#!/usr/bin/env python3
"""通过 Mihomo API 查询和更新 Provider，并安全归档本地缓存。

修改时间：2026-07-28 18:15:12 +08:00
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request
from collections.abc import MutableMapping
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from archive import ArchiveStore
from config import ConfigError, load_config, print_provider_list, provider_list


class CacheError(Exception):
    """Provider 缓存操作失败。"""


def providers_from(root: MutableMapping[str, Any]) -> MutableMapping[str, Any]:
    providers = root.get("proxy-providers")
    if providers is None:
        return {}
    if not isinstance(providers, MutableMapping):
        raise CacheError("proxy-providers 必须是 YAML 对象")
    return providers


def controller_url(root: MutableMapping[str, Any]) -> str:
    controller = str(root.get("external-controller") or "127.0.0.1:9090")
    if not controller.startswith(("http://", "https://")):
        controller = f"http://{controller}"
    return controller.rstrip("/")


def provider_subscription_details(root: MutableMapping[str, Any]) -> dict[str, str]:
    request = urllib.request.Request(f"{controller_url(root)}/providers/proxies")
    secret = str(root.get("secret") or "")
    if secret:
        request.add_header("Authorization", f"Bearer {secret}")
    # Controller 是本地管理接口，禁止继承当前 shell 的代理环境变量。
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        with opener.open(request, timeout=3) as response:
            payload = json.load(response)
    except (OSError, ValueError, urllib.error.HTTPError, urllib.error.URLError):
        return {}

    providers = payload.get("providers") if isinstance(payload, dict) else None
    if not isinstance(providers, dict):
        return {}
    details: dict[str, str] = {}
    for name, provider in providers.items():
        if not isinstance(provider, dict):
            continue
        info = provider.get("subscriptionInfo") or provider.get("subscription-info")
        if not isinstance(info, dict):
            continue
        details[str(name)] = subscription_label(info)
    return details


def subscription_label(info: dict[str, Any]) -> str:
    normalized = {str(key).lower(): value for key, value in info.items()}
    upload = integer_value(normalized.get("upload"))
    download = integer_value(normalized.get("download"))
    total = integer_value(normalized.get("total"))
    expire = integer_value(normalized.get("expire"))

    remaining = "-"
    if total > 0:
        remaining = f"{max(total - max(upload + download, 0), 0) / 1_000_000_000:.1f} GB"
    expire_text = "-"
    if expire > 0:
        try:
            expire_text = datetime.fromtimestamp(expire, timezone.utc).strftime("%Y-%m-%d")
        except (OverflowError, OSError, ValueError):
            pass
    return f"remain {remaining} expire {expire_text}"


def integer_value(value: Any) -> int:
    if isinstance(value, bool):
        return 0
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    try:
        return int(str(value))
    except (TypeError, ValueError, OverflowError):
        try:
            return int(float(value))
        except (TypeError, ValueError, OverflowError):
            return 0


def update_http_providers(root: MutableMapping[str, Any], names: list[str]) -> tuple[list[str], list[str]]:
    providers = providers_from(root)
    selected = names or [
        str(name)
        for name, value in providers.items()
        if isinstance(value, MutableMapping) and value.get("type") == "http"
    ]
    if not selected:
        return ["没有可更新的 HTTP Provider"], []

    base_url = controller_url(root)
    secret = str(root.get("secret") or "")
    # Controller 是本地管理接口，禁止继承当前 shell 的代理环境变量。
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    successes: list[str] = []
    failures: list[str] = []
    for name in selected:
        provider = providers.get(name)
        if not isinstance(provider, MutableMapping):
            failures.append(f"{name}: Provider 不存在")
            continue
        if provider.get("type") != "http":
            failures.append(f"{name}: 不是 HTTP Provider")
            continue
        encoded_name = urllib.parse.quote(name, safe="")
        request = urllib.request.Request(
            f"{base_url}/providers/proxies/{encoded_name}",
            data=b"",
            method="PUT",
        )
        if secret:
            request.add_header("Authorization", f"Bearer {secret}")
        try:
            with opener.open(request, timeout=30) as response:
                if response.status < 200 or response.status >= 300:
                    raise CacheError(f"HTTP {response.status}")
            successes.append(f"Provider updated: {name}")
        except (OSError, urllib.error.HTTPError, urllib.error.URLError, CacheError) as exc:
            failures.append(f"{name}: {exc}")
    return successes, failures


def lexical_path(path: Path) -> Path:
    """比较受管路径时不解析软链接，避免把链接目标误认为 providers 内文件。"""
    return Path(os.path.abspath(path))


def clean_provider_cache(root: MutableMapping[str, Any], home: Path, logs_dir: Path) -> tuple[Path | None, int]:
    provider_root = lexical_path(home / "providers")
    if not provider_root.is_dir():
        return None, 0

    preserved: set[Path] = set()
    for provider in providers_from(root).values():
        if not isinstance(provider, MutableMapping) or provider.get("type") != "file":
            continue
        raw_path = provider.get("path")
        if not raw_path:
            continue
        path = Path(str(raw_path)).expanduser()
        managed = lexical_path(path if path.is_absolute() else home / path)
        try:
            managed.relative_to(provider_root)
        except ValueError:
            continue
        preserved.add(managed)

    candidates = [
        path
        for path in provider_root.rglob("*")
        if (path.is_file() or path.is_symlink()) and lexical_path(path) not in preserved
    ]
    archive_dir = ArchiveStore(logs_dir).archive_provider_files(provider_root, candidates)
    return archive_dir, len(candidates)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="clashdev Provider 缓存管理")
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--home", required=True, type=Path)
    parser.add_argument("--logs-dir", required=True, type=Path)
    sub = parser.add_subparsers(dest="command", required=True)
    update = sub.add_parser("update", help="通过 Mihomo API 更新 HTTP Provider")
    update.add_argument("names", nargs="*")
    sub.add_parser("list", help="列出 Provider 和订阅余额")
    sub.add_parser("clean", help="归档 HTTP Provider 缓存和孤立文件")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        root = load_config(args.config.expanduser().resolve())
        if args.command == "update":
            successes, failures = update_http_providers(root, args.names)
            for message in successes:
                print(message)
            for message in failures:
                print(f"Provider update failed: {message}", file=sys.stderr)
            return 1 if failures else 0

        if args.command == "list":
            print_provider_list(provider_list(root), provider_subscription_details(root))
            return 0

        archive_dir, count = clean_provider_cache(
            root,
            args.home.expanduser().resolve(),
            args.logs_dir.expanduser().resolve(),
        )
        if archive_dir is None:
            print("Provider cache already clean")
        else:
            print(f"Provider cache archived: {archive_dir}")
            print(f"Files archived: {count}")
        return 0
    except (CacheError, ConfigError, OSError) as exc:
        print(f"cache: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
