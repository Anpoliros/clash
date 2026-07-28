#!/usr/bin/env python3
"""为 File Provider 提供可回滚的原子文件导入。

修改时间：2026-07-22 17:26:42 +08:00
"""

from __future__ import annotations

import filecmp
import os
import shutil
import tempfile
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator


class ProviderFileError(Exception):
    """Provider 源文件或目标路径不合法。"""


def managed_provider_path(home: Path, raw_path: str | None, name: str) -> tuple[str, Path]:
    """返回适合写入 YAML 的相对路径及其绝对目标路径。"""
    home = home.expanduser().resolve()
    requested = Path(raw_path).expanduser() if raw_path else Path("providers") / f"{name}.yaml"
    target = requested if requested.is_absolute() else home / requested
    target = target.resolve()
    try:
        relative = target.relative_to(home)
    except ValueError as exc:
        raise ProviderFileError(f"Provider 目标必须位于 Mihomo HomeDir 内：{target}") from exc
    if target == home:
        raise ProviderFileError("Provider 目标必须是文件路径")
    return f"./{relative.as_posix()}", target


class ProviderFileInstall:
    """在上下文中替换 Provider 文件，异常退出时恢复原文件。"""

    def __init__(self, source: Path, target: Path):
        self.source = source.expanduser().resolve()
        self.target = target.expanduser().resolve()
        if not self.source.is_file():
            raise ProviderFileError(f"Provider 源文件不存在：{self.source}")

    def needs_update(self) -> bool:
        if self.source == self.target:
            return False
        return not self.target.is_file() or not filecmp.cmp(self.source, self.target, shallow=False)

    @contextmanager
    def activate(self) -> Iterator[bool]:
        """原子安装文件；后续配置校验失败时回滚。"""
        if not self.needs_update():
            yield False
            return

        self.target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        fd, raw_staged = tempfile.mkstemp(prefix=f".{self.target.name}.", suffix=".tmp", dir=self.target.parent)
        staged = Path(raw_staged)
        rollback = self.target.parent / f".{self.target.name}.{os.getpid()}.rollback"
        had_target = self.target.exists()
        try:
            with self.source.open("rb") as input_stream, os.fdopen(fd, "wb") as output_stream:
                shutil.copyfileobj(input_stream, output_stream)
                output_stream.flush()
                os.fsync(output_stream.fileno())
            staged.chmod(0o600)
            if rollback.exists():
                rollback.unlink()
            if had_target:
                os.replace(self.target, rollback)
            os.replace(staged, self.target)
            try:
                yield True
            except BaseException:
                if self.target.exists():
                    self.target.unlink()
                if had_target and rollback.exists():
                    os.replace(rollback, self.target)
                raise
            else:
                if rollback.exists():
                    rollback.unlink()
        finally:
            if staged.exists():
                staged.unlink()
            if rollback.exists():
                if not self.target.exists():
                    os.replace(rollback, self.target)
                else:
                    rollback.unlink()
