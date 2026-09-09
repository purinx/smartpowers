from __future__ import annotations

import importlib.util
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path


SCRIPT = (
    Path(__file__).parents[1]
    / "plugins"
    / "smartpowers"
    / "skills"
    / "smartpowers"
    / "scripts"
    / "init_project.py"
)
SPEC = importlib.util.spec_from_file_location("init_project", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class InitProjectTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.jst = timezone(timedelta(hours=9))

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_creates_timestamped_project_metadata(self) -> None:
        project = MODULE.create_project(
            self.root,
            "OAuth Refresh",
            now=datetime(2026, 9, 8, 18, 30, 45, tzinfo=self.jst),
        )

        self.assertEqual(project.name, "20260908-183045-oauth-refresh")
        metadata = (project / "PROJECT.md").read_text(encoding="utf-8")
        self.assertIn(f"Project: {project.name}", metadata)
        self.assertIn("Status: Active", metadata)
        self.assertIn("Created: 2026-09-08T18:30:45+09:00", metadata)

    def test_same_slug_at_different_times_creates_distinct_projects(self) -> None:
        first = MODULE.create_project(
            self.root,
            "Auth",
            "auth",
            datetime(2026, 9, 8, 18, 30, 45, tzinfo=self.jst),
        )
        second = MODULE.create_project(
            self.root,
            "Auth",
            "auth",
            datetime(2026, 9, 8, 18, 31, 45, tzinfo=self.jst),
        )

        self.assertNotEqual(first, second)
        self.assertTrue(first.is_dir())
        self.assertTrue(second.is_dir())

    def test_non_ascii_title_requires_explicit_slug(self) -> None:
        with self.assertRaisesRegex(MODULE.ProjectInitError, "--slug"):
            MODULE.create_project(
                self.root,
                "認証更新",
                now=datetime(2026, 9, 8, 18, 30, 45, tzinfo=self.jst),
            )


if __name__ == "__main__":
    unittest.main()
