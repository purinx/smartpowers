from __future__ import annotations

import http.client
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
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
        self.root = Path(self.temp.name) / "repo"
        # Comments and logs go to a state directory outside the repository.
        self.env = {**os.environ, "SMARTPOWERS_STATE_HOME": str(Path(self.temp.name) / "state")}
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
            env=self.env,
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

    def port(self) -> int:
        line = self.process.stdout.readline()
        self.assertTrue(line.startswith("Smartpowers board: http://localhost:"), line)
        self.assertTrue(line.strip().endswith(f"#/{PROJECT_ID}"))
        return int(line.split("localhost:")[1].split("/")[0])

    def request(self, port: int, path: str, headers: dict | None = None, method="GET", body=None):
        conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
        conn.request(method, path, body=body, headers=headers or {})
        response = conn.getresponse()
        data = response.read()
        conn.close()
        return response.status, data

    def board(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            [str(LAUNCHER), *args, "--root", str(self.root), "--project", PROJECT_ID],
            capture_output=True,
            text=True,
            timeout=60,
            env=self.env,
        )

    def test_serves_project_and_rejects_foreign_host(self) -> None:
        port = self.port()
        status, body = self.request(port, f"/api/projects/{PROJECT_ID}")
        self.assertEqual(status, 200)
        task = json.loads(body)["tasks"][0]
        self.assertEqual(task["title"], "Infrastructure foundations")
        self.assertEqual(task["statusKey"], "in_progress")

        status, _ = self.request(port, "/api/projects", {"Host": "evil.example"})
        self.assertEqual(status, 403)

    def test_comment_feedback_loop_with_the_agent(self) -> None:
        port = self.port()
        status, body = self.request(
            port,
            f"/api/projects/{PROJECT_ID}/tasks/01/comments",
            {"Origin": f"http://localhost:{port}", "Content-Type": "text/plain; charset=utf-8"},
            method="POST",
            body="Use the existing HTTP client".encode("utf-8"),
        )
        self.assertEqual(status, 201, body)
        comment_id = json.loads(body)["id"]

        inbox = self.board("inbox")
        self.assertEqual(inbox.returncode, 0, inbox.stderr)
        self.assertIn(f"Comment {comment_id} on task 01", inbox.stdout)
        self.assertIn("Use the existing HTTP client", inbox.stdout)
        # Each comment is delivered once.
        self.assertIn("No new comments", self.board("inbox").stdout)

        reply = self.board("reply", "--comment", comment_id, "--resolve", "Switched to it")
        self.assertEqual(reply.returncode, 0, reply.stderr)
        logged = self.board("log", "--task", "01", "Swapped the client; tests pass")
        self.assertEqual(logged.returncode, 0, logged.stderr)

        _, body = self.request(port, f"/api/projects/{PROJECT_ID}")
        task = json.loads(body)["tasks"][0]
        root, answer = task["comments"]
        self.assertEqual(root["status"], "addressed")
        self.assertEqual((answer["author"], answer["replyTo"]), ("agent", comment_id))
        self.assertEqual(task["log"][0]["message"], "Swapped the client; tests pass")
        # Nothing was written inside the repository.
        self.assertFalse(any(p.name.endswith(".tmp") or "comments" in p.parts for p in self.root.rglob("*")))

    def test_subagent_receives_comments_live(self) -> None:
        port = self.port()
        listener = subprocess.Popen(
            [str(LAUNCHER), "inbox", "--root", str(self.root), "--project", PROJECT_ID,
             "--task", "01", "--wait", "--timeout", "60"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=self.env,
        )
        try:
            # The board shows the task as having a listening agent once it connects.
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                _, body = self.request(port, f"/api/projects/{PROJECT_ID}")
                if json.loads(body)["tasks"][0]["listening"]:
                    break
                time.sleep(0.2)
            else:
                self.fail("the inbox never connected to the board")

            sent = time.monotonic()
            status, _ = self.request(
                port,
                f"/api/projects/{PROJECT_ID}/tasks/01/comments",
                {"Origin": f"http://localhost:{port}"},
                method="POST",
                body="Rename the module".encode("utf-8"),
            )
            self.assertEqual(status, 201)
            out, err = listener.communicate(timeout=30)
            self.assertEqual(listener.returncode, 0, err)
            self.assertIn("Rename the module", out)
            self.assertIn("--task 01 --wait", out)
            # Pushed over SSE, not found by polling.
            self.assertLess(time.monotonic() - sent, 5)
        finally:
            if listener.poll() is None:
                listener.kill()
                listener.communicate()

    def test_next_lists_parallel_tasks_in_plan_order(self) -> None:
        tasks = self.root / "docs" / "smartpowers" / "projects" / PROJECT_ID / "tasks"
        write(tasks / "01_infra_foundations.md",
              f"# 01: Infrastructure foundations\n\nStatus: Done\nProject: {PROJECT_ID}\nDepends on: None\n")
        for number, name in (("02", "domain"), ("03", "api")):
            write(tasks / f"{number}_{name}.md",
                  f"# {number}: {name}\n\nStatus: Planned\nProject: {PROJECT_ID}\n"
                  "Depends on: 01\nParallel group: B\n")
        write(tasks / "PLAN.md", "| 01 | [a](01_infra_foundations.md) |\n| 03 | [c](03_api.md) |\n| 02 | [b](02_domain.md) |\n")

        result = self.board("next")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Start now", result.stdout)
        self.assertLess(result.stdout.index("03  api"), result.stdout.index("02  domain"))

    def test_rejects_non_loopback_host(self) -> None:
        result = subprocess.run(
            [str(LAUNCHER), "--root", str(self.root), "--host", "0.0.0.0", "--no-open"],
            capture_output=True,
            text=True,
            timeout=30,
            env=self.env,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("loopback", result.stderr)


if __name__ == "__main__":
    unittest.main()
