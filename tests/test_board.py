from __future__ import annotations

import http.client
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


WINDOWS = sys.platform == "win32"
LAUNCHER = (
    Path(__file__).parents[1]
    / "plugins"
    / "smartpowers"
    / "skills"
    / "smartpowers"
    / "scripts"
    / ("board.cmd" if WINDOWS else "board")
)
PROJECT_ID = "20260908-183045-oauth-refresh"


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


@unittest.skipUnless(shutil.which("cargo"), "the board is built with cargo")
class BoardLauncherTests(unittest.TestCase):
    """Black-box checks of the board built from source through its launcher.

    Parsing and server internals are covered by `cargo test` in the board crate.
    """

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        project = self.root / "docs" / "smartpowers" / "projects" / PROJECT_ID
        write(project / "PROJECT.md", f"# OAuth refresh\n\nStatus: Active\nProject: {PROJECT_ID}\n")
        write(
            project / "tasks" / "01_infra_foundations.md",
            "# 01: Infrastructure foundations\n\nStatus: In Progress\n"
            f"Project: {PROJECT_ID}\nRequirements: REQ-001\nDepends on: None\n",
        )
        self.process = subprocess.Popen(
            [str(LAUNCHER), "--root", str(self.root), "--project", PROJECT_ID, "--no-open"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    def tearDown(self) -> None:
        if WINDOWS:
            # board.cmd runs the server as a child of cmd.exe; stop the whole tree.
            subprocess.run(
                ["taskkill", "/T", "/F", "/PID", str(self.process.pid)],
                capture_output=True,
            )
        else:
            self.process.terminate()
        self.process.wait(timeout=5)
        self.process.stdout.close()
        self.process.stderr.close()
        self.temp.cleanup()

    def request(self, port: int, path: str, headers: dict | None = None):
        conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
        conn.request("GET", path, headers=headers or {})
        response = conn.getresponse()
        body = response.read()
        conn.close()
        return response.status, body

    def test_serves_project_and_rejects_foreign_host(self) -> None:
        line = self.process.stdout.readline()
        self.assertTrue(line.startswith("Smartpowers board: http://localhost:"), line)
        self.assertTrue(line.strip().endswith(f"#/{PROJECT_ID}"))
        port = int(line.split("localhost:")[1].split("/")[0])

        status, body = self.request(port, f"/api/projects/{PROJECT_ID}")
        self.assertEqual(status, 200)
        task = json.loads(body)["tasks"][0]
        self.assertEqual(task["title"], "Infrastructure foundations")
        self.assertEqual(task["statusKey"], "in_progress")

        status, _ = self.request(port, "/api/projects", {"Host": "evil.example"})
        self.assertEqual(status, 403)

    def test_rejects_non_loopback_host(self) -> None:
        result = subprocess.run(
            [str(LAUNCHER), "--root", str(self.root), "--host", "0.0.0.0", "--no-open"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("loopback", result.stderr)


if __name__ == "__main__":
    unittest.main()
