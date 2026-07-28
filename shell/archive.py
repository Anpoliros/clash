#!/usr/bin/env python3
"""管理 clashdev 的运行日志、配置备份和归档目录。

修改时间：2026-07-22 17:30:29 +08:00
"""

from __future__ import annotations

import argparse
import os
import shutil
import sys
import tempfile
import time
from pathlib import Path


class ArchiveError(Exception):
    """归档目录或文件操作失败。"""


class ArchiveStore:
    """集中管理 logs 下的持久化文件。"""

    def __init__(self, root: Path):
        self.root = root.expanduser().resolve()
        self.runtime_dir = self.root / "mihomo"
        self.backup_dir = self.root / "backup"
        self.provider_archive_dir = self.root / "providers"

    @staticmethod
    def _ensure_private_dir(path: Path) -> None:
        path.mkdir(parents=True, exist_ok=True, mode=0o700)
        path.chmod(0o700)

    @staticmethod
    def _timestamp() -> str:
        return time.strftime("%y%m%d_%H%M%S")

    def _unique_path(self, directory: Path, prefix: str, suffix: str) -> Path:
        candidate = directory / f"{prefix}_{self._timestamp()}{suffix}"
        index = 1
        while candidate.exists() or candidate.is_symlink():
            candidate = directory / f"{prefix}_{self._timestamp()}_{index}{suffix}"
            index += 1
        return candidate

    @staticmethod
    def _sync_directory(path: Path) -> None:
        directory_fd = os.open(path, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)

    def create_runtime_log(self) -> Path:
        """创建本次运行日志，并原子更新 current.log 软链接。"""
        self._ensure_private_dir(self.root)
        self._ensure_private_dir(self.runtime_dir)
        runtime = self._unique_path(self.runtime_dir, "runtime", ".log")
        fd = os.open(runtime, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        os.close(fd)

        current = self.runtime_dir / "current.log"
        temporary_link = self.runtime_dir / f".current.{os.getpid()}.tmp"
        try:
            if temporary_link.exists() or temporary_link.is_symlink():
                temporary_link.unlink()
            temporary_link.symlink_to(runtime.name)
            os.replace(temporary_link, current)
            self._sync_directory(self.runtime_dir)
        finally:
            if temporary_link.exists() or temporary_link.is_symlink():
                temporary_link.unlink()
        return runtime

    def backup_config(self, source: Path) -> Path:
        """将配置原子复制到 logs/backup，保留所有历史版本。"""
        source = source.expanduser().resolve()
        if not source.is_file():
            raise ArchiveError(f"待备份配置不存在：{source}")
        self._ensure_private_dir(self.root)
        self._ensure_private_dir(self.backup_dir)
        target = self._unique_path(self.backup_dir, "config", ".yaml")
        fd, raw_temporary = tempfile.mkstemp(prefix=".config_", suffix=".tmp", dir=self.backup_dir)
        temporary = Path(raw_temporary)
        try:
            with source.open("rb") as input_stream, os.fdopen(fd, "wb") as output_stream:
                shutil.copyfileobj(input_stream, output_stream)
                output_stream.flush()
                os.fsync(output_stream.fileno())
            temporary.chmod(0o600)
            os.replace(temporary, target)
            self._sync_directory(self.backup_dir)
            return target
        finally:
            if temporary.exists():
                temporary.unlink()

    def config_backups(self) -> list[Path]:
        if not self.backup_dir.is_dir():
            return []
        return sorted(self.backup_dir.glob("config_*.yaml"), reverse=True)

    def archive_provider_files(self, provider_root: Path, files: list[Path]) -> Path | None:
        """保留相对目录结构移动 Provider 文件，失败时尽量整体回滚。"""
        if not files:
            return None
        provider_root = provider_root.expanduser().absolute()
        self._ensure_private_dir(self.root)
        self._ensure_private_dir(self.provider_archive_dir)
        archive_dir = self._unique_path(self.provider_archive_dir, "providers", "")
        archive_dir.mkdir(mode=0o700)
        moved: list[tuple[Path, Path]] = []
        try:
            for source in sorted(files):
                source = source.absolute()
                try:
                    relative = source.relative_to(provider_root)
                except ValueError as exc:
                    raise ArchiveError(f"拒绝归档 providers 目录之外的文件：{source}") from exc
                target = archive_dir / relative
                target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
                shutil.move(str(source), str(target))
                if target.is_file() and not target.is_symlink():
                    target.chmod(0o600)
                moved.append((source, target))
        except BaseException:
            for source, target in reversed(moved):
                source.parent.mkdir(parents=True, exist_ok=True)
                if target.exists() or target.is_symlink():
                    shutil.move(str(target), str(source))
            shutil.rmtree(archive_dir, ignore_errors=True)
            raise

        # 仅删除本次移动后留下的空目录，不处理 providers 根目录本身。
        directories = sorted(
            (path for path in provider_root.rglob("*") if path.is_dir()),
            key=lambda path: len(path.parts),
            reverse=True,
        )
        for directory in directories:
            try:
                directory.rmdir()
            except OSError:
                pass
        self._sync_directory(self.provider_archive_dir)
        return archive_dir


# #----命令行入口----
def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="clashdev 日志和备份归档")
    parser.add_argument("--logs-dir", required=True, type=Path)
    sub = parser.add_subparsers(dest="command", required=True)

    runtime = sub.add_parser("runtime")
    runtime.add_argument("action", choices=["create", "current"])

    backup = sub.add_parser("backup")
    backup.add_argument("action", choices=["create", "list"])
    backup.add_argument("--source", type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    store = ArchiveStore(args.logs_dir)
    try:
        if args.command == "runtime":
            path = store.create_runtime_log() if args.action == "create" else store.runtime_dir / "current.log"
            print(path)
            return 0
        if args.action == "list":
            for path in store.config_backups():
                print(path)
            return 0
        if args.source is None:
            raise ArchiveError("backup create 必须提供 --source")
        print(store.backup_config(args.source))
        return 0
    except (ArchiveError, OSError) as exc:
        print(f"archive: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
