"""Prevent LAB release tooling from depending on a fixed service-worker cache version."""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("autotrace_pages_builder", ROOT / "scripts" / "build-pages-preview.py")
BUILDER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILDER)


class PagesLabBuilderTests(unittest.TestCase):
    def test_each_cache_version_is_isolated_and_stable_root_is_unchanged(self):
        for version in (9, 10, 35):
            with self.subTest(version=version), tempfile.TemporaryDirectory() as temp:
                base = Path(temp)
                stable, output = base / "stable", base / "output"
                stable.mkdir()
                (stable / "vendor").mkdir()
                (stable / "vendor" / "xlsx.mini.min.js").write_text("/* synthetic fixture */")
                (stable / "index.html").write_text(
                    "<title>AutoTrace · Gestión de taller</title><body><p>Stable</p></body>",
                    encoding="utf-8")
                (stable / "app.js").write_text(
                    "const DB_NAME = 'autotrace-v1';\n", encoding="utf-8")
                (stable / "sw.js").write_text(
                    "const PREFIX = 'autotrace-shell-';\n"
                    f"const CACHE = 'autotrace-shell-v{version}';\n",
                    encoding="utf-8")
                (stable / "manifest.webmanifest").write_text(
                    json.dumps({"name": "AutoTrace", "id": "./"}), encoding="utf-8")
                original = {p.name: p.read_bytes() for p in stable.glob("*") if p.is_file()}
                BUILDER.build(stable, stable, output)
                for name, contents in original.items():
                    self.assertEqual((output / name).read_bytes(), contents)
                self.assertIn(
                    f"const CACHE = 'autotrace-lab-shell-v{version}';",
                    (output / "lab" / "sw.js").read_text(encoding="utf-8"))
                self.assertIn(
                    "const DB_NAME = 'autotrace-lab-v1';",
                    (output / "lab" / "app.js").read_text(encoding="utf-8"))

    def test_ambiguous_cache_declaration_fails_closed(self):
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            stable = base / "stable"
            stable.mkdir()
            (stable / "vendor").mkdir()
            (stable / "vendor" / "xlsx.mini.min.js").write_text("")
            (stable / "index.html").write_text(
                "<title>AutoTrace · Gestión de taller</title><body></body>")
            (stable / "app.js").write_text("const DB_NAME = 'autotrace-v1';")
            (stable / "sw.js").write_text(
                "const PREFIX = 'autotrace-shell-';\n"
                "const CACHE = 'autotrace-shell-v10';\n"
                "const CACHE = 'autotrace-shell-v11';\n")
            (stable / "manifest.webmanifest").write_text("{}")
            with self.assertRaisesRegex(ValueError, "exactly one"):
                BUILDER.build(stable, stable, base / "output")


if __name__ == "__main__":
    unittest.main()
