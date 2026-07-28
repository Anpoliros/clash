#!/usr/bin/env python3
"""clashdev 配置管理：round-trip 修改 mihomo YAML，并提供原子写入。

修改时间：2026-07-28 18:15:12 +08:00
"""

from __future__ import annotations

import argparse
import io
import os
import subprocess
import sys
import tempfile
from collections.abc import MutableMapping, MutableSequence
from contextlib import nullcontext
from pathlib import Path
from typing import Any, Callable

# #----内置依赖----
SCRIPT_DIR = Path(__file__).resolve().parent
VENDOR_DIR = SCRIPT_DIR / "vendor"
sys.path.insert(0, str(VENDOR_DIR))

from ruamel.yaml import YAML  # noqa: E402
from ruamel.yaml.comments import CommentedMap, CommentedSeq  # noqa: E402
from ruamel.yaml.error import YAMLError  # noqa: E402

from archive import ArchiveStore  # noqa: E402
from provider_file import ProviderFileError, ProviderFileInstall, managed_provider_path  # noqa: E402


class ConfigError(Exception):
    """用户配置或配置操作不合法。"""


# #----YAML 读写----
def yaml_codec() -> YAML:
    yaml = YAML(typ="rt", pure=True)
    yaml.preserve_quotes = True
    yaml.allow_duplicate_keys = False
    yaml.width = 4096
    yaml.indent(mapping=2, sequence=4, offset=2)
    return yaml


def load_config(path: Path) -> MutableMapping[str, Any]:
    if not path.is_file():
        raise ConfigError(f"配置文件不存在：{path}")
    try:
        with path.open("r", encoding="utf-8") as stream:
            root = yaml_codec().load(stream)
    except (OSError, YAMLError) as exc:
        raise ConfigError(f"YAML 解析失败：{exc}") from exc
    if not isinstance(root, MutableMapping):
        raise ConfigError("配置顶层必须是 YAML 对象")
    return root


def is_blank(value: Any) -> bool:
    return value is None or not str(value).strip()


def validate_config(root: MutableMapping[str, Any], semantic: bool = True) -> None:
    providers = root.get("proxy-providers")
    if providers is not None and not isinstance(providers, MutableMapping):
        raise ConfigError("proxy-providers 必须是 YAML 对象")

    provider_names: set[str] = set()
    provider_paths: dict[str, str] = {}
    for raw_name, provider in (providers or {}).items():
        name = str(raw_name)
        if is_blank(name):
            raise ConfigError("Provider 名称不能为空")
        if not isinstance(provider, MutableMapping):
            raise ConfigError(f"Provider {name} 必须是 YAML 对象")
        if not semantic:
            provider_names.add(name)
            continue
        provider_type = provider.get("type")
        if provider_type not in {"http", "file", "inline"}:
            raise ConfigError(f"Provider {name} 的 type 必须是 http/file/inline")
        if provider_type == "http" and is_blank(provider.get("url")):
            raise ConfigError(f"HTTP Provider {name} 缺少 url")
        if provider_type == "file" and is_blank(provider.get("path")):
            raise ConfigError(f"File Provider {name} 缺少 path")
        if provider_type == "inline" and not isinstance(provider.get("payload"), MutableSequence):
            raise ConfigError(f"Inline Provider {name} 缺少 payload 列表")
        provider_path = provider.get("path")
        if not is_blank(provider_path):
            normalized_path = str(provider_path)
            if normalized_path in provider_paths:
                raise ConfigError(
                    f"Provider {name} 与 {provider_paths[normalized_path]} 使用了相同 path：{normalized_path}"
                )
            provider_paths[normalized_path] = name
        provider_names.add(name)

    groups = root.get("proxy-groups")
    if groups is None:
        return
    if not isinstance(groups, MutableSequence):
        raise ConfigError("proxy-groups 必须是 YAML 列表")
    for index, group in enumerate(groups):
        if not isinstance(group, MutableMapping):
            raise ConfigError(f"proxy-groups[{index}] 必须是 YAML 对象")
        group_name = str(group.get("name") or index)
        uses = group.get("use")
        if uses is not None and not isinstance(uses, MutableSequence):
            raise ConfigError(f"策略组 {group_name} 的 use 必须是列表")
        if not semantic:
            continue
        if uses is not None:
            missing = [str(name) for name in uses if str(name) not in provider_names]
            if missing:
                raise ConfigError(f"策略组 {group_name} 引用了不存在的 Provider：{', '.join(missing)}")

        proxies = group.get("proxies")
        has_use = isinstance(uses, MutableSequence) and len(uses) > 0
        has_proxies = isinstance(proxies, MutableSequence) and len(proxies) > 0
        includes_all = any(
            group.get(field) is True
            for field in ("include-all", "include-all-proxies", "include-all-providers")
        )
        if not has_use and not has_proxies and not includes_all:
            raise ConfigError(f"策略组 {group_name} 缺少非空 use 或 proxies")


def explicit_key(mapping: MutableMapping[str, Any], key: str) -> bool:
    items = getattr(mapping, "non_merged_items", None)
    if items is None:
        return key in mapping
    return any(item_key == key for item_key, _ in items())


def replace_sequence(mapping: MutableMapping[str, Any], key: str, values: list[Any]) -> None:
    current = mapping.get(key)
    if explicit_key(mapping, key) and isinstance(current, MutableSequence):
        current[:] = values
    else:
        mapping[key] = CommentedSeq(values)


def append_unique(mapping: MutableMapping[str, Any], key: str, value: str) -> None:
    current = mapping.get(key)
    values = list(current) if isinstance(current, MutableSequence) else []
    if value in [str(item) for item in values]:
        return
    values.append(value)
    replace_sequence(mapping, key, values)


def remove_value(mapping: MutableMapping[str, Any], key: str, value: str) -> None:
    current = mapping.get(key)
    if not isinstance(current, MutableSequence):
        return
    if value not in [str(item) for item in current]:
        return
    values = [item for item in current if str(item) != value]
    replace_sequence(mapping, key, values)


def validate_with_mihomo(candidate: Path, validator: Path | None, home: Path | None) -> None:
    if validator is None:
        return
    if home is None:
        raise ConfigError("使用 mihomo 校验时必须提供 --home")
    if not validator.is_file() or not os.access(validator, os.X_OK):
        raise ConfigError(f"mihomo 校验器不可执行：{validator}")
    result = subprocess.run(
        [str(validator), "-t", "-d", str(home), "-f", str(candidate)],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if result.returncode != 0:
        output = result.stdout.strip() or "mihomo 未返回错误详情"
        raise ConfigError(f"mihomo 配置校验失败：\n{output}")


def update_config(
    path: Path,
    mutate: Callable[[MutableMapping[str, Any]], list[str]],
    validator: Path | None,
    home: Path | None,
    logs_dir: Path,
    file_install: ProviderFileInstall | None = None,
) -> list[str]:
    root = load_config(path)
    # 允许首次 add 修复模板中的空策略组；写回前仍会执行完整语义校验。
    validate_config(root, semantic=False)
    before = io.StringIO()
    yaml_codec().dump(root, before)
    messages = mutate(root)
    validate_config(root)
    after = io.StringIO()
    yaml_codec().dump(root, after)
    config_changed = after.getvalue() != before.getvalue()
    if not config_changed and (file_install is None or not file_install.needs_update()):
        return ["Config unchanged", *messages]

    fd, raw_candidate = tempfile.mkstemp(prefix=f".{path.name}.", suffix=".tmp", dir=path.parent)
    candidate = Path(raw_candidate)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as stream:
            yaml_codec().dump(root, stream)
            stream.flush()
            os.fsync(stream.fileno())
        candidate.chmod(path.stat().st_mode & 0o777)
        validate_config(load_config(candidate))
        install_context = file_install.activate() if file_install is not None else nullcontext(False)
        with install_context as file_changed:
            validate_with_mihomo(candidate, validator, home)
            if config_changed:
                backup = ArchiveStore(logs_dir).backup_config(path)
                os.replace(candidate, path)
                directory_fd = os.open(path.parent, os.O_RDONLY)
                try:
                    os.fsync(directory_fd)
                finally:
                    os.close(directory_fd)
                prefix = [f"Backup: {backup}"]
            else:
                prefix = ["Config unchanged"]
            if file_changed:
                messages.append(f"File imported: {file_install.target}")
            return [*prefix, *messages]
    finally:
        if candidate.exists():
            candidate.unlink()


# #----配置查询----
def scalar_text(value: Any, default: str = "") -> str:
    if value is None:
        return default
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (MutableMapping, MutableSequence)):
        raise ConfigError("请求的配置字段不是标量")
    return str(value)


def get_field(root: MutableMapping[str, Any], field: str) -> str:
    tun = root.get("tun") if isinstance(root.get("tun"), MutableMapping) else {}
    providers = root.get("proxy-providers")
    provider_names = list(providers) if isinstance(providers, MutableMapping) else []
    fields = {
        "port": scalar_text(root.get("port"), "-"),
        "mixed-port": scalar_text(root.get("mixed-port"), "-"),
        "socks-port": scalar_text(root.get("socks-port"), "-"),
        "proxy-port": scalar_text(root.get("mixed-port") or root.get("port"), "7890"),
        "external-controller": scalar_text(root.get("external-controller"), "-"),
        "controller": scalar_text(root.get("external-controller"), "127.0.0.1:9090"),
        "secret": scalar_text(root.get("secret")),
        "tun-enabled": scalar_text(tun.get("enable"), "false"),
        "provider-names": ", ".join(str(name) for name in provider_names),
    }
    if field not in fields:
        raise ConfigError(f"不支持的配置字段：{field}")
    return fields[field]


def provider_list(root: MutableMapping[str, Any]) -> list[tuple[str, str, str]]:
    providers = root.get("proxy-providers")
    if providers is None:
        return []
    if not isinstance(providers, MutableMapping):
        raise ConfigError("proxy-providers 必须是 YAML 对象")
    rows = []
    for raw_name, provider in providers.items():
        if not isinstance(provider, MutableMapping):
            raise ConfigError(f"Provider {raw_name} 必须是 YAML 对象")
        provider_type = str(provider.get("type") or "-")
        source = provider.get("url") or provider.get("path") or "inline"
        rows.append((str(raw_name), provider_type, str(source)))
    return rows


def print_provider_list(
    rows: list[tuple[str, str, str]],
    subscriptions: MutableMapping[str, str] | None = None,
) -> None:
    subscriptions = subscriptions or {}
    name_width = max([4, *(len(row[0]) for row in rows)])
    type_width = max([4, *(len(row[1]) for row in rows)])
    subscription_rows = [subscriptions.get(name, "remain - expire -") for name, _, _ in rows]
    subscription_width = max([12, *(len(value) for value in subscription_rows)])
    print(
        f"{'NAME':<{name_width}}   {'TYPE':<{type_width}}   "
        f"{'SUBSCRIPTION':<{subscription_width}}   SOURCE"
    )
    print(
        f"{'----':<{name_width}}   {'----':<{type_width}}   "
        f"{'------------':<{subscription_width}}   ------"
    )
    for (name, provider_type, source), subscription in zip(rows, subscription_rows):
        print(
            f"{name:<{name_width}}   {provider_type:<{type_width}}   "
            f"{subscription:<{subscription_width}}   {source}"
        )


# #----Provider 修改----
def provider_name(raw_name: str) -> str:
    name = raw_name.strip()
    if (
        not name
        or name in {".", ".."}
        or "/" in name
        or "\\" in name
        or any(ord(character) < 32 for character in name)
    ):
        raise ConfigError("Provider 名称不能为空，也不能包含路径分隔符或控制字符")
    return name


def ensure_providers(root: MutableMapping[str, Any]) -> MutableMapping[str, Any]:
    providers = root.get("proxy-providers")
    if providers is None:
        providers = CommentedMap()
        root["proxy-providers"] = providers
    if not isinstance(providers, MutableMapping):
        raise ConfigError("proxy-providers 必须是 YAML 对象")
    return providers


def provider_definition(args: argparse.Namespace) -> CommentedMap:
    name = provider_name(args.name)
    prefix = f"[{name}]"
    health_check = CommentedMap(
        {
            "enable": True,
            "url": "https://cp.cloudflare.com",
            "interval": 300,
            "timeout": 1000,
            "tolerance": 100,
        }
    )
    provider_type = "file" if args.file else "http"
    provider = CommentedMap({"type": provider_type})
    if provider_type == "http":
        if is_blank(args.url):
            raise ConfigError("HTTP Provider 必须提供非空的 -u/--url")
        provider["interval"] = 3600
        provider["health-check"] = health_check
        provider["url"] = args.url
        provider["path"] = args.provider_path
    else:
        provider["path"] = args.provider_path
        provider["health-check"] = health_check
    provider["override"] = CommentedMap({"additional-prefix": prefix})
    return provider


def add_provider(root: MutableMapping[str, Any], args: argparse.Namespace) -> list[str]:
    name = provider_name(args.name)
    providers = ensure_providers(root)
    providers[name] = provider_definition(args)

    anchor_use = root.get("use")
    if isinstance(anchor_use, MutableMapping):
        append_unique(anchor_use, "use", name)

    target_names = args.group or ["自动选择", "全部节点"]
    updated_groups: list[str] = []
    groups = root.get("proxy-groups")
    if isinstance(groups, MutableSequence):
        for group in groups:
            if not isinstance(group, MutableMapping):
                continue
            group_name = str(group.get("name") or "")
            if group_name in target_names:
                append_unique(group, "use", name)
                updated_groups.append(group_name)

    messages = [f"Provider added: {name}", f"Path: {providers[name].get('path', '-')}"]
    if not updated_groups:
        messages.append("Warning: 未找到目标策略组，Provider 已添加但尚未被 proxy-groups 引用")
    return messages


def remove_provider(root: MutableMapping[str, Any], args: argparse.Namespace) -> list[str]:
    name = provider_name(args.name)
    providers = ensure_providers(root)
    if name not in providers:
        raise ConfigError(f"Provider 不存在：{name}")
    del providers[name]

    anchor_use = root.get("use")
    if isinstance(anchor_use, MutableMapping):
        remove_value(anchor_use, "use", name)
    groups = root.get("proxy-groups")
    if isinstance(groups, MutableSequence):
        for group in groups:
            if isinstance(group, MutableMapping):
                remove_value(group, "use", name)
    return [f"Provider removed: {name}"]


def set_provider_url(root: MutableMapping[str, Any], args: argparse.Namespace) -> list[str]:
    name = provider_name(args.name)
    providers = ensure_providers(root)
    provider = providers.get(name)
    if not isinstance(provider, MutableMapping):
        raise ConfigError(f"Provider 不存在：{name}")
    if provider.get("type") != "http":
        raise ConfigError(f"只有 HTTP Provider 可以修改 URL：{name}")
    if is_blank(args.url):
        raise ConfigError("HTTP Provider URL 不能为空")
    provider["url"] = args.url
    return [f"Provider URL updated: {name}"]


def prepare_provider_add(args: argparse.Namespace, home: Path | None) -> ProviderFileInstall | None:
    name = provider_name(args.name)
    if home is None:
        raise ConfigError("添加 Provider 时必须提供 --home")
    try:
        args.provider_path, target = managed_provider_path(home, args.path, name)
        if args.file:
            return ProviderFileInstall(Path(args.file), target)
    except ProviderFileError as exc:
        raise ConfigError(str(exc)) from exc
    return None


# #----通用字段修改----
def set_tun_enabled(root: MutableMapping[str, Any], args: argparse.Namespace) -> list[str]:
    tun = root.get("tun")
    if tun is None:
        tun = CommentedMap(
            {
                "stack": "system",
                "auto-route": True,
                "auto-detect-interface": True,
            }
        )
        root["tun"] = tun
    if not isinstance(tun, MutableMapping):
        raise ConfigError("tun 必须是 YAML 对象")
    enabled = args.value == "true"
    tun["enable"] = enabled
    return [f"tun.enable: {'true' if enabled else 'false'}"]


# #----CLI----
def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="clashdev mihomo 配置管理")
    parser.add_argument("--config", required=True, type=Path, help="mihomo config.yaml 路径")
    parser.add_argument("--home", type=Path, help="mihomo 工作目录")
    parser.add_argument("--validator", type=Path, help="用于 -t 校验的 mihomo 二进制")
    parser.add_argument("--logs-dir", type=Path, help="日志与配置备份根目录")
    sub = parser.add_subparsers(dest="command", required=True)

    get = sub.add_parser("get", help="读取 clash shell 所需字段")
    get.add_argument("field")

    sub.add_parser("validate", help="校验 YAML 和 Provider 引用")

    set_value = sub.add_parser("set", help="安全修改 clash shell 所需字段")
    set_value.add_argument("field", choices=["tun-enabled"])
    set_value.add_argument("value", choices=["true", "false"])

    provider = sub.add_parser("provider", help="管理 proxy-providers")
    provider_sub = provider.add_subparsers(dest="provider_command", required=True)

    add = provider_sub.add_parser("add", help="添加或更新 Provider")
    add.add_argument("name")
    source = add.add_mutually_exclusive_group(required=True)
    source.add_argument("-u", "--url")
    source.add_argument("-f", "--file")
    add.add_argument("-p", "--path")
    add.add_argument("-g", "--group", action="append", help="要引用该 Provider 的策略组，可重复")

    remove = provider_sub.add_parser("rm", aliases=["remove"], help="移除 Provider")
    remove.add_argument("name")

    set_url = provider_sub.add_parser("set-url", help="修改 HTTP Provider 的订阅链接")
    set_url.add_argument("name")
    set_url.add_argument("-u", "--url", required=True)

    provider_sub.add_parser("ls", aliases=["list"], help="列出 Provider")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    config_path = args.config.expanduser().resolve()
    home = args.home.expanduser().resolve() if args.home else None
    validator = args.validator.expanduser().resolve() if args.validator else None
    logs_dir = args.logs_dir.expanduser().resolve() if args.logs_dir else None
    try:
        if args.command == "get":
            print(get_field(load_config(config_path), args.field))
            return 0
        if args.command == "validate":
            validate_config(load_config(config_path))
            print(f"Config valid: {config_path}")
            return 0
        if args.command == "set":
            if logs_dir is None:
                raise ConfigError("修改配置时必须提供 --logs-dir")
            messages = update_config(config_path, lambda root: set_tun_enabled(root, args), validator, home, logs_dir)
            print("\n".join(messages))
            return 0
        if args.provider_command in {"ls", "list"}:
            print_provider_list(provider_list(load_config(config_path)))
            return 0
        if args.provider_command == "add":
            if logs_dir is None:
                raise ConfigError("修改配置时必须提供 --logs-dir")
            file_install = prepare_provider_add(args, home)
            messages = update_config(
                config_path,
                lambda root: add_provider(root, args),
                validator,
                home,
                logs_dir,
                file_install,
            )
        elif args.provider_command == "set-url":
            if logs_dir is None:
                raise ConfigError("修改配置时必须提供 --logs-dir")
            messages = update_config(config_path, lambda root: set_provider_url(root, args), validator, home, logs_dir)
        else:
            if logs_dir is None:
                raise ConfigError("修改配置时必须提供 --logs-dir")
            messages = update_config(config_path, lambda root: remove_provider(root, args), validator, home, logs_dir)
        print("\n".join(messages))
        return 0
    except (ConfigError, OSError) as exc:
        print(f"config: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
