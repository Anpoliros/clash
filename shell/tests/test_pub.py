#!/usr/bin/env python3
"""JSON 发布任务回归测试；临时仓库仅写入 shell/tests。

修改时间：2026-07-22 20:17:37 +08:00
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

TEST_DIR = Path(__file__).resolve().parent
SHELL_DIR = TEST_DIR.parent
PUB = SHELL_DIR / "pub.py"


class PublishTaskTest(unittest.TestCase):
    def setUp(self) -> None:
        self.workspace = Path(tempfile.mkdtemp(prefix="pub-", dir=TEST_DIR))
        self.source = self.workspace / "source"
        self.target = self.workspace / "target"
        self.source.mkdir()
        self.target.mkdir()
        subprocess.run(["git", "init", "-q", str(self.target)], check=True)

    def tearDown(self) -> None:
        shutil.rmtree(self.workspace)

    def write_task(self, **overrides: object) -> Path:
        task: dict[str, object] = {
            "version": 1,
            "source_root": "source",
            "publish_paths": ["keep.txt", "tree"],
            "exclude_paths": ["tree/private.txt"],
            "clean_target_paths": ["keep.txt", "tree"],
            "file_mappings": [],
        }
        task.update(overrides)
        path = self.workspace / "publish.json"
        path.write_text(json.dumps(task), encoding="utf-8")
        return path

    def run_pub(
        self,
        task: Path,
        check: bool = True,
        target: Path | None = None,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(PUB), str(task), "--target", str(target or self.target)],
            check=check,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def test_json_task_controls_copy_exclude_and_clean(self) -> None:
        (self.source / "tree").mkdir()
        (self.source / "keep.txt").write_text("new\n", encoding="utf-8")
        (self.source / "tree" / "public.txt").write_text("public\n", encoding="utf-8")
        (self.source / "tree" / "private.txt").write_text("private\n", encoding="utf-8")
        (self.target / "keep.txt").write_text("old\n", encoding="utf-8")
        (self.target / "tree").mkdir()
        (self.target / "tree" / "stale.txt").write_text("stale\n", encoding="utf-8")

        result = self.run_pub(self.write_task())

        self.assertIn("Published to:", result.stdout)
        self.assertEqual((self.target / "keep.txt").read_text(encoding="utf-8"), "new\n")
        self.assertTrue((self.target / "tree" / "public.txt").is_file())
        self.assertFalse((self.target / "tree" / "private.txt").exists())
        self.assertFalse((self.target / "tree" / "stale.txt").exists())

    def test_json_task_rejects_path_escape(self) -> None:
        result = self.run_pub(self.write_task(publish_paths=["../private"]), check=False)

        self.assertEqual(result.returncode, 1)
        self.assertIn("非法路径", result.stderr)

    def test_target_cannot_be_inside_source(self) -> None:
        nested_target = self.source / "target"
        nested_target.mkdir()
        subprocess.run(["git", "init", "-q", str(nested_target)], check=True)

        result = self.run_pub(self.write_task(), check=False, target=nested_target)

        self.assertEqual(result.returncode, 1)
        self.assertIn("不能互相包含", result.stderr)

    def test_file_mapping_replaces_excluded_private_file(self) -> None:
        (self.source / "private").mkdir()
        (self.source / "examples").mkdir()
        (self.source / "private" / "config.yaml").write_text("secret: real\n", encoding="utf-8")
        (self.source / "examples" / "config.yaml").write_text("secret: example\n", encoding="utf-8")
        task = self.write_task(
            publish_paths=["private", "examples"],
            exclude_paths=["private/config.yaml"],
            clean_target_paths=["private", "examples"],
            file_mappings=[{"source": "examples/config.yaml", "target": "private/config.yaml"}],
        )

        self.run_pub(task)

        public_config = self.target / "private" / "config.yaml"
        self.assertEqual(public_config.read_text(encoding="utf-8"), "secret: example\n")


if __name__ == "__main__":
    unittest.main()
