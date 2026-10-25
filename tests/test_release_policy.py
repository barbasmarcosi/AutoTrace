"""Regression tests for AutoTrace release policy checks (no real customer data)."""
import json
import shutil
import tempfile
import unittest
from pathlib import Path
from scripts.check_desktop_release_policy import ROOT, FILES, failures

class ReleasePolicyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="autotrace-policy-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for rel in FILES:
            target = self.root / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / rel, target)

    def test_current_release_candidate_policy(self):
        self.assertEqual(failures(ROOT), [])

    def test_rejects_broad_tauri_permissions(self):
        p = self.root / "desktop/src-tauri/capabilities/default.json"
        data = json.loads(p.read_text(encoding="utf-8"))
        data["permissions"].append("shell:allow-execute")
        p.write_text(json.dumps(data), encoding="utf-8")
        self.assertTrue(any("capability" in e for e in failures(self.root)))

    def test_rejects_broad_drive_scope(self):
        p = self.root / "desktop/src-tauri/src/drive_oauth.rs"
        data = p.read_text(encoding="utf-8")
        p.write_text(data.replace("https://www.googleapis.com/auth/drive.file",
                                  "https://www.googleapis.com/auth/drive"), encoding="utf-8")
        self.assertTrue(any("scope" in e for e in failures(self.root)))

    def test_rejects_plaintext_sqlite_in_installer_artifacts(self):
        p = self.root / ".github/workflows/desktop-installer.yml"
        text = p.read_text(encoding="utf-8")
        self.assertIn("*.sha256.txt", text)
        p.write_text(text.replace("*.sha256.txt", "*.sqlite3"), encoding="utf-8")
        problems = failures(self.root)
        self.assertTrue(any("private workshop data" in e for e in problems))

    def test_rejects_unsafe_eval_in_tauri_csp(self):
        p = self.root / "desktop/src-tauri/tauri.conf.json"
        config = json.loads(p.read_text(encoding="utf-8"))
        config["app"]["security"]["csp"] += "; script-src 'unsafe-eval'"
        p.write_text(json.dumps(config), encoding="utf-8")
        self.assertTrue(any("unsafe-eval" in e for e in failures(self.root)))

    def test_rejects_vulnerable_crypto_bigint_regression(self):
        p = self.root / "desktop/src-tauri/Cargo.lock"
        p.write_text(
            p.read_text(encoding="utf-8")
            + '\n[[package]]\nname = "crypto-bigint"\nversion = "0.5.5"\n',
            encoding="utf-8",
        )
        self.assertTrue(any("RUSTSEC-2026-0387" in e for e in failures(self.root)))

    def test_rejects_unprotected_ci_secrets(self):
        p = self.root / ".github/workflows/desktop-installer.yml"
        p.write_text(p.read_text(encoding="utf-8")+"\n# secrets.USER_TOKEN\n",encoding="utf-8")
        self.assertTrue(any("Unsafe GitHub Actions" in e for e in failures(self.root)))

if __name__ == "__main__":
    unittest.main()
