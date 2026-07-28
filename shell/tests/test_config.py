#!/usr/bin/env python3
"""配置工具回归测试；所有临时文件都位于 shell/tests。

修改时间：2026-07-28 18:15:12 +08:00
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

TEST_DIR = Path(__file__).resolve().parent
SHELL_DIR = TEST_DIR.parent
PROJECT_DIR = SHELL_DIR.parent
CONFIG_TOOL = SHELL_DIR / "config.py"
CLASH = SHELL_DIR / "clash"
SETSHELL = SHELL_DIR / "setshell.sh"
FIXTURE = TEST_DIR / "fixtures" / "config.yaml"

sys.path.insert(0, str(SHELL_DIR / "vendor"))
from ruamel.yaml import YAML  # noqa: E402


class ConfigToolTest(unittest.TestCase):
    def setUp(self) -> None:
        self.workspace = Path(tempfile.mkdtemp(prefix="workspace-", dir=TEST_DIR))
        self.config = self.workspace / "config.yaml"
        self.logs = self.workspace / "logs"
        shutil.copy2(FIXTURE, self.config)

    def tearDown(self) -> None:
        shutil.rmtree(self.workspace)

    def run_config(self, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(CONFIG_TOOL),
                "--config",
                str(self.config),
                "--home",
                str(self.workspace),
                "--logs-dir",
                str(self.logs),
                *args,
            ],
            check=check,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def load(self):
        yaml = YAML(typ="rt", pure=True)
        with self.config.open(encoding="utf-8") as stream:
            return yaml.load(stream)

    def shell_env(self) -> dict[str, str]:
        fake_bin = self.workspace / "bin"
        fake_bin.mkdir(exist_ok=True)
        fake_ps = fake_bin / "ps"
        fake_ps.write_text("#!/usr/bin/env sh\nexit 0\n", encoding="utf-8")
        fake_ps.chmod(0o755)

        # yq 即使碰巧安装在系统中也不应被调用；一旦调用便让测试立即失败。
        fake_yq = fake_bin / "yq"
        fake_yq.write_text("#!/usr/bin/env sh\nexit 99\n", encoding="utf-8")
        fake_yq.chmod(0o755)
        env = os.environ.copy()
        env.update(
            {
                "CLASH_DIR": str(self.workspace),
                "CLASH_CONFIG": str(self.config),
                "CLASH_BIN": str(self.workspace / "missing-mihomo"),
                "CLASH_PID": str(self.workspace / "mihomo.pid"),
                "CLASH_LOG": str(self.workspace / "mihomo.log"),
                "CLASH_LOGS_DIR": str(self.logs),
                "PATH": f"{fake_bin}:/usr/bin:/bin",
            }
        )
        return env

    def test_add_http_preserves_comments_anchors_and_file_provider(self) -> None:
        result = self.run_config(
            "provider",
            "add",
            "new-http",
            "-u",
            "https://example.invalid/new",
        )

        text = self.config.read_text(encoding="utf-8")
        root = self.load()
        self.assertIn("Provider added: new-http", result.stdout)
        self.assertIn("# fixture 顶层注释必须保留", text)
        self.assertIn("&p", text)
        self.assertIn("<<: *p", text)
        self.assertEqual(root["proxy-providers"]["local-file"]["type"], "file")
        self.assertEqual(root["proxy-providers"]["new-http"]["path"], "./providers/new-http.yaml")
        self.assertIn("new-http", root["use"]["use"])
        self.assertIn("new-http", root["proxy-groups"][2]["use"])
        self.assertEqual(len(list((self.logs / "backup").glob("config_*.yaml"))), 1)

    def test_add_file_provider_imports_to_default_managed_path(self) -> None:
        source = self.workspace / "source.yaml"
        source.write_text("proxies: []\n", encoding="utf-8")
        self.run_config("provider", "add", "new-file", "-f", str(source))

        provider = self.load()["proxy-providers"]["new-file"]
        self.assertEqual(provider["type"], "file")
        self.assertEqual(provider["path"], "./providers/new-file.yaml")
        self.assertNotIn("url", provider)
        self.assertEqual((self.workspace / "providers" / "new-file.yaml").read_bytes(), source.read_bytes())

    def test_add_file_provider_supports_custom_managed_path(self) -> None:
        source = self.workspace / "source.yaml"
        source.write_text("proxies: []\n", encoding="utf-8")
        self.run_config(
            "provider",
            "add",
            "custom-file",
            "-f",
            str(source),
            "-p",
            "./providers/nested/custom.yaml",
        )

        provider = self.load()["proxy-providers"]["custom-file"]
        self.assertEqual(provider["path"], "./providers/nested/custom.yaml")
        self.assertTrue((self.workspace / "providers" / "nested" / "custom.yaml").is_file())

    def test_provider_target_cannot_escape_home_or_duplicate_path(self) -> None:
        source = self.workspace / "source.yaml"
        source.write_text("proxies: []\n", encoding="utf-8")
        before = self.config.read_bytes()

        escaped = self.run_config(
            "provider",
            "add",
            "escaped",
            "-f",
            str(source),
            "-p",
            "../escaped.yaml",
            check=False,
        )
        self.assertNotEqual(escaped.returncode, 0)
        self.assertIn("HomeDir", escaped.stderr)

        duplicate = self.run_config(
            "provider",
            "add",
            "duplicate",
            "-u",
            "https://example.invalid/duplicate",
            "-p",
            "./providers/old-http.yaml",
            check=False,
        )
        self.assertNotEqual(duplicate.returncode, 0)
        self.assertIn("相同 path", duplicate.stderr)
        self.assertEqual(self.config.read_bytes(), before)

    def test_add_first_provider_repairs_empty_public_template(self) -> None:
        self.config.write_text(
            """\
proxy-providers: {}
proxy-groups:
  - name: 自动选择
    type: url-test
    use: []
  - name: 全部节点
    type: select
    use: []
rules:
  - MATCH,全部节点
""",
            encoding="utf-8",
        )

        self.run_config(
            "provider",
            "add",
            "first",
            "-u",
            "https://example.invalid/first",
        )

        root = self.load()
        self.assertIn("first", root["proxy-providers"])
        self.assertEqual(root["proxy-groups"][0]["use"], ["first"])
        self.assertEqual(root["proxy-groups"][1]["use"], ["first"])

    def test_remove_provider_cleans_use_references(self) -> None:
        self.run_config("provider", "rm", "old-http")

        root = self.load()
        self.assertNotIn("old-http", root["proxy-providers"])
        self.assertNotIn("old-http", root["use"]["use"])
        for group in root["proxy-groups"]:
            self.assertNotIn("old-http", group.get("use", []))

    def test_set_tun_enabled_uses_atomic_config_update(self) -> None:
        before = self.config.read_text(encoding="utf-8")
        self.run_config("set", "tun-enabled", "true")

        text = self.config.read_text(encoding="utf-8")
        self.assertIn("# fixture 顶层注释必须保留", text)
        self.assertIn("&p", text)
        self.assertIs(self.load()["tun"]["enable"], True)
        self.assertNotEqual(text, before)
        self.assertEqual(len(list((self.logs / "backup").glob("config_*.yaml"))), 1)

    def test_unchanged_tun_value_does_not_create_backup(self) -> None:
        result = self.run_config("set", "tun-enabled", "false")

        self.assertIn("Config unchanged", result.stdout)
        self.assertEqual(list((self.logs / "backup").glob("config_*.yaml")), [])

    def test_missing_file_provider_source_keeps_original_unchanged(self) -> None:
        before = self.config.read_bytes()
        result = self.run_config(
            "provider",
            "add",
            "broken",
            "-f",
            str(self.workspace / "missing.yaml"),
            check=False,
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("源文件不存在", result.stderr)
        self.assertEqual(self.config.read_bytes(), before)
        self.assertEqual(list((self.logs / "backup").glob("config_*.yaml")), [])

    def test_failed_mihomo_validator_rolls_back_candidate(self) -> None:
        validator = self.workspace / "fake-mihomo"
        validator.write_text("#!/usr/bin/env sh\necho validator rejected\nexit 1\n", encoding="utf-8")
        validator.chmod(0o755)
        before = self.config.read_bytes()
        result = subprocess.run(
            [
                sys.executable,
                str(CONFIG_TOOL),
                "--config",
                str(self.config),
                "--home",
                str(self.workspace),
                "--validator",
                str(validator),
                "--logs-dir",
                str(self.logs),
                "provider",
                "add",
                "new-http",
                "-u",
                "https://example.invalid/new",
            ],
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("validator rejected", result.stderr)
        self.assertEqual(self.config.read_bytes(), before)
        self.assertEqual(list((self.logs / "backup").glob("config_*.yaml")), [])

    def test_failed_validator_rolls_back_imported_provider_file(self) -> None:
        source = self.workspace / "source.yaml"
        source.write_text("proxies: []\n", encoding="utf-8")
        target = self.workspace / "providers" / "local-file.yaml"
        target.parent.mkdir()
        target.write_text("proxies:\n  - name: old\n", encoding="utf-8")
        before_config = self.config.read_bytes()
        before_target = target.read_bytes()
        validator = self.workspace / "fake-mihomo"
        validator.write_text("#!/usr/bin/env sh\nexit 1\n", encoding="utf-8")
        validator.chmod(0o755)

        result = subprocess.run(
            [
                sys.executable,
                str(CONFIG_TOOL),
                "--config",
                str(self.config),
                "--home",
                str(self.workspace),
                "--validator",
                str(validator),
                "--logs-dir",
                str(self.logs),
                "provider",
                "add",
                "local-file",
                "-f",
                str(source),
            ],
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.config.read_bytes(), before_config)
        self.assertEqual(target.read_bytes(), before_target)

    def test_set_url_updates_only_http_provider(self) -> None:
        self.run_config(
            "provider",
            "set-url",
            "old-http",
            "-u",
            "https://example.invalid/new-url",
        )
        self.assertEqual(
            self.load()["proxy-providers"]["old-http"]["url"],
            "https://example.invalid/new-url",
        )

        result = self.run_config(
            "provider",
            "set-url",
            "local-file",
            "-u",
            "https://example.invalid/invalid",
            check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("只有 HTTP Provider", result.stderr)

    def test_shell_config_add_uses_isolated_workspace_without_yq(self) -> None:
        result = subprocess.run(
            [
                "bash",
                str(CLASH),
                "config",
                "add",
                "from-shell",
                "-u",
                "https://example.invalid/shell",
            ],
            cwd=PROJECT_DIR,
            env=self.shell_env(),
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        self.assertEqual(result.returncode, 0, msg=f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}")
        self.assertIn("Provider added: from-shell", result.stdout)
        self.assertIn("from-shell", self.load()["proxy-providers"])

    def test_shell_status_reads_config_without_yq(self) -> None:
        result = subprocess.run(
            ["bash", str(CLASH), "status"],
            cwd=PROJECT_DIR,
            env=self.shell_env(),
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        self.assertIn("port=7890", result.stdout)
        self.assertIn("controller: 127.0.0.1:19090", result.stdout)
        self.assertIn("providers:  old-http, local-file", result.stdout)

    def test_proxy_env_resyncs_after_config_port_changes(self) -> None:
        (self.workspace / "mihomo.pid").write_text(str(os.getpid()), encoding="utf-8")
        result = subprocess.run(
            [
                "bash",
                "-c",
                (
                    f"source {CLASH!s} env >/dev/null; "
                    "printf 'before=%s\\n' \"$http_proxy\"; "
                    "sed -i 's/port: 7890/port: 8899/' \"$CLASH_CONFIG\"; "
                    "_clash_auto_env; "
                    "printf 'after=%s\\n' \"$http_proxy\""
                ),
            ],
            cwd=PROJECT_DIR,
            env=self.shell_env(),
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        self.assertIn("before=127.0.0.1:7890", result.stdout)
        self.assertIn("after=127.0.0.1:8899", result.stdout)

    def test_setshell_registers_zsh_and_bash_prompt_hooks(self) -> None:
        bashrc = self.workspace / ".bashrc"
        zshrc = self.workspace / ".zshrc"
        bashrc.write_text("# bash fixture\n", encoding="utf-8")
        zshrc.write_text("# zsh fixture\n", encoding="utf-8")
        env = self.shell_env()
        env["HOME"] = str(self.workspace)

        subprocess.run(
            ["bash", str(SETSHELL), "install"],
            cwd=PROJECT_DIR,
            env=env,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        bash_text = bashrc.read_text(encoding="utf-8")
        zsh_text = zshrc.read_text(encoding="utf-8")
        self.assertIn("clash env", bash_text)
        self.assertIn("clash env", zsh_text)

        bash_result = subprocess.run(
            [
                "bash",
                "-c",
                (
                    f"source {bashrc!s}; "
                    "declare -F _clash_auto_env >/dev/null; "
                    "declare -p PROMPT_COMMAND"
                ),
            ],
            cwd=PROJECT_DIR,
            env=env,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.assertIn("_clash_auto_env", bash_result.stdout)

        if shutil.which("zsh"):
            subprocess.run(
                [
                    "zsh",
                    "-c",
                    (
                        f"source {zshrc!s}; "
                        "typeset -f _clash_auto_env >/dev/null; "
                        "(( ${precmd_functions[(Ie)_clash_auto_env]} > 0 ))"
                    ),
                ],
                cwd=PROJECT_DIR,
                env=env,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

    def test_shell_cache_commands_use_isolated_workspace(self) -> None:
        clean = subprocess.run(
            ["bash", str(CLASH), "config", "cache-clean"],
            cwd=PROJECT_DIR,
            env=self.shell_env(),
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.assertIn("already clean", clean.stdout)

        update = subprocess.run(
            ["bash", str(CLASH), "config", "cache-update"],
            cwd=PROJECT_DIR,
            env=self.shell_env(),
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self.assertNotEqual(update.returncode, 0)
        self.assertIn("mihomo 未运行", update.stderr)

    @unittest.skipUnless(shutil.which("zsh"), "zsh 未安装")
    def test_zsh_config_list_uses_embedded_python_tool(self) -> None:
        result = subprocess.run(
            ["zsh", str(CLASH), "config", "ls"],
            cwd=PROJECT_DIR,
            env=self.shell_env(),
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        self.assertIn("old-http", result.stdout)
        self.assertIn("local-file", result.stdout)
        self.assertIn("remain - expire -", result.stdout)


if __name__ == "__main__":
    unittest.main()
