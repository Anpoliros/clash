#!/usr/bin/env python3
"""Provider 查询和缓存回归测试；网络和文件均隔离在 shell/tests。

修改时间：2026-07-28 18:15:12 +08:00
"""

from __future__ import annotations

import shutil
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

TEST_DIR = Path(__file__).resolve().parent
SHELL_DIR = TEST_DIR.parent
FIXTURE = TEST_DIR / "fixtures" / "config.yaml"
sys.path.insert(0, str(SHELL_DIR))

from config import load_config  # noqa: E402
from provider_cache import (  # noqa: E402
    clean_provider_cache,
    provider_subscription_details,
    subscription_label,
    update_http_providers,
)


class ProviderHandler(BaseHTTPRequestHandler):
    requests: list[tuple[str, str | None]] = []

    def do_GET(self) -> None:  # noqa: N802
        self.requests.append((self.path, self.headers.get("Authorization")))
        body = b"""{
            "providers": {
                "old-http": {
                    "subscriptionInfo": {
                        "Upload": 10000000000,
                        "Download": 13400000000,
                        "Total": 100000000000,
                        "Expire": 1788134400
                    }
                },
                "local-file": {}
            }
        }"""
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_PUT(self) -> None:  # noqa: N802
        self.requests.append((self.path, self.headers.get("Authorization")))
        self.send_response(204)
        self.end_headers()

    def log_message(self, format: str, *args: object) -> None:
        pass


class ProviderCacheTest(unittest.TestCase):
    def setUp(self) -> None:
        self.workspace = Path(tempfile.mkdtemp(prefix="workspace-", dir=TEST_DIR))
        self.config = self.workspace / "config.yaml"
        self.logs = self.workspace / "logs"
        shutil.copy2(FIXTURE, self.config)
        ProviderHandler.requests = []

    def tearDown(self) -> None:
        shutil.rmtree(self.workspace)

    def test_update_uses_mihomo_provider_api_and_authorization(self) -> None:
        server = ThreadingHTTPServer(("127.0.0.1", 0), ProviderHandler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            root = load_config(self.config)
            root["external-controller"] = f"127.0.0.1:{server.server_port}"
            successes, failures = update_http_providers(root, [])
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

        self.assertEqual(failures, [])
        self.assertEqual(successes, ["Provider updated: old-http"])
        self.assertEqual(
            ProviderHandler.requests,
            [("/providers/proxies/old-http", "Bearer fixture-secret")],
        )

    def test_update_rejects_file_and_unknown_provider(self) -> None:
        root = load_config(self.config)
        successes, failures = update_http_providers(root, ["local-file", "missing"])

        self.assertEqual(successes, [])
        self.assertIn("local-file: 不是 HTTP Provider", failures)
        self.assertIn("missing: Provider 不存在", failures)

    def test_subscription_details_use_mihomo_api_and_authorization(self) -> None:
        server = ThreadingHTTPServer(("127.0.0.1", 0), ProviderHandler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            root = load_config(self.config)
            root["external-controller"] = f"127.0.0.1:{server.server_port}"
            details = provider_subscription_details(root)
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

        self.assertEqual(details["old-http"], "remain 76.6 GB expire 2026-08-31")
        self.assertNotIn("local-file", details)
        self.assertEqual(
            ProviderHandler.requests,
            [("/providers/proxies", "Bearer fixture-secret")],
        )

    def test_subscription_label_handles_missing_values(self) -> None:
        self.assertEqual(subscription_label({}), "remain - expire -")

    def test_clean_archives_cache_and_orphans_but_preserves_file_provider(self) -> None:
        providers = self.workspace / "providers"
        (providers / "nested").mkdir(parents=True)
        (providers / "old-http.yaml").write_text("proxies: []\n", encoding="utf-8")
        (providers / "local-file.yaml").write_text("proxies: []\n", encoding="utf-8")
        (providers / "nested" / "orphan.yaml").write_text("proxies: []\n", encoding="utf-8")
        backup = self.logs / "backup" / "config_260722_173029.yaml"
        backup.parent.mkdir(parents=True)
        backup.write_text("port: 7890\n", encoding="utf-8")

        archive_dir, count = clean_provider_cache(load_config(self.config), self.workspace, self.logs)

        self.assertEqual(count, 2)
        self.assertIsNotNone(archive_dir)
        assert archive_dir is not None
        self.assertTrue((archive_dir / "old-http.yaml").is_file())
        self.assertTrue((archive_dir / "nested" / "orphan.yaml").is_file())
        self.assertEqual(archive_dir.stat().st_mode & 0o777, 0o700)
        self.assertEqual((archive_dir / "old-http.yaml").stat().st_mode & 0o777, 0o600)
        self.assertTrue((providers / "local-file.yaml").is_file())
        self.assertFalse((providers / "old-http.yaml").exists())
        self.assertTrue(backup.is_file())


if __name__ == "__main__":
    unittest.main()
