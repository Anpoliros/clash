#!/usr/bin/env python3
"""归档模块回归测试；临时内容仅写入 shell/tests。

修改时间：2026-07-22 17:23:37 +08:00
"""

from __future__ import annotations

import shutil
import sys
import tempfile
import unittest
from pathlib import Path

TEST_DIR = Path(__file__).resolve().parent
SHELL_DIR = TEST_DIR.parent
sys.path.insert(0, str(SHELL_DIR))

from archive import ArchiveStore  # noqa: E402


class ArchiveStoreTest(unittest.TestCase):
    def setUp(self) -> None:
        self.workspace = Path(tempfile.mkdtemp(prefix="workspace-", dir=TEST_DIR))
        self.store = ArchiveStore(self.workspace / "logs")

    def tearDown(self) -> None:
        shutil.rmtree(self.workspace)

    def test_runtime_logs_are_unique_and_current_points_to_latest(self) -> None:
        first = self.store.create_runtime_log()
        second = self.store.create_runtime_log()

        current = self.store.runtime_dir / "current.log"
        self.assertNotEqual(first, second)
        self.assertEqual(current.resolve(), second)
        self.assertEqual(first.stat().st_mode & 0o777, 0o600)
        self.assertEqual(self.store.root.stat().st_mode & 0o777, 0o700)
        self.assertEqual(self.store.runtime_dir.stat().st_mode & 0o777, 0o700)

    def test_config_backups_are_private_and_keep_all_versions(self) -> None:
        config = self.workspace / "config.yaml"
        config.write_text("port: 7890\n", encoding="utf-8")
        first = self.store.backup_config(config)
        config.write_text("port: 7891\n", encoding="utf-8")
        second = self.store.backup_config(config)

        self.assertNotEqual(first, second)
        self.assertEqual(first.read_text(encoding="utf-8"), "port: 7890\n")
        self.assertEqual(second.read_text(encoding="utf-8"), "port: 7891\n")
        self.assertEqual(second.stat().st_mode & 0o777, 0o600)
        self.assertEqual(self.store.config_backups(), [second, first])


if __name__ == "__main__":
    unittest.main()
