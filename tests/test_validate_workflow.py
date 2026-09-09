from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT = (
    Path(__file__).parents[1]
    / "plugins"
    / "smartpowers"
    / "skills"
    / "smartpowers"
    / "scripts"
    / "validate_workflow.py"
)
SPEC = importlib.util.spec_from_file_location("validate_workflow", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class ValidateWorkflowTests(unittest.TestCase):
    project_id = "20260908-183045-auth-refresh"

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.project = (
            self.root / "docs" / "smartpowers" / "projects" / self.project_id
        )
        (self.project / "design_docs").mkdir(parents=True)
        (self.project / "tasks").mkdir()
        self.write(
            "PROJECT.md",
            f"# Auth refresh\n\nProject: {self.project_id}\nStatus: Active\n",
        )

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write(self, relative: str, text: str) -> None:
        path = self.project / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def valid_project(self) -> None:
        self.write(
            "PRD.md",
            f"# PRD\n\nStatus: Approved\nProject: {self.project_id}\n"
            "Approved: 2026-09-08 by user\n\nREQ-001: login\nNFR-001: latency\n",
        )
        self.write(
            "design_docs/FIRST_BUILD.md",
            f"# Design\n\nStatus: Approved\nProject: {self.project_id}\n"
            "Approved: 2026-09-08 by user\n\nREQ-001 via auth flow\n"
            "NFR-001 via cache\n",
        )
        self.write(
            "tasks/PLAN.md",
            f"# Plan\n\nStatus: Approved\nProject: {self.project_id}\n"
            "Approved: 2026-09-08 by user\n",
        )
        self.write(
            "tasks/01_auth.md",
            f"# Auth\n\nStatus: Planned\nProject: {self.project_id}\n"
            "Requirements: REQ-001, NFR-001\n",
        )

    def test_valid_plan(self) -> None:
        self.valid_project()
        selected = MODULE.resolve_project(self.root, self.project_id)
        requirements = MODULE.validate_requirements(selected, True)
        MODULE.validate_design(selected, requirements, True)
        MODULE.validate_plan(selected, requirements)

    def test_design_reports_missing_requirement(self) -> None:
        self.valid_project()
        self.write(
            "design_docs/FIRST_BUILD.md",
            f"# Design\n\nStatus: Draft\nProject: {self.project_id}\n\n"
            "REQ-001 via auth flow\n",
        )
        requirements = MODULE.validate_requirements(self.project, True)
        with self.assertRaisesRegex(MODULE.ValidationError, "NFR-001"):
            MODULE.validate_design(self.project, requirements, False)

    def test_plan_rejects_invalid_task_filename(self) -> None:
        self.valid_project()
        (self.project / "tasks" / "01_auth.md").unlink()
        self.write(
            "tasks/auth-task.md",
            f"# Auth\n\nStatus: Planned\nProject: {self.project_id}\n"
            "Requirements: REQ-001, NFR-001\n",
        )
        requirements = MODULE.validate_requirements(self.project, True)
        with self.assertRaisesRegex(MODULE.ValidationError, "filenames"):
            MODULE.validate_plan(self.project, requirements)

    def test_approved_status_requires_approval_record(self) -> None:
        self.valid_project()
        self.write(
            "tasks/PLAN.md",
            f"# Plan\n\nStatus: Approved\nProject: {self.project_id}\n",
        )
        requirements = MODULE.validate_requirements(self.project, True)
        with self.assertRaisesRegex(MODULE.ValidationError, "Approved:"):
            MODULE.validate_plan(self.project, requirements)

    def test_draft_plan_can_be_validated_before_review(self) -> None:
        self.valid_project()
        self.write(
            "tasks/PLAN.md",
            f"# Plan\n\nStatus: Draft\nProject: {self.project_id}\n",
        )
        requirements = MODULE.validate_requirements(self.project, True)
        MODULE.validate_plan(self.project, requirements, require_approved=False)

    def test_rejects_artifact_from_another_project(self) -> None:
        self.valid_project()
        self.write(
            "PRD.md",
            "# PRD\n\nStatus: Draft\nProject: 20260908-190000-other-work\n"
            "REQ-001: other\n",
        )
        with self.assertRaisesRegex(MODULE.ValidationError, "belongs to project"):
            MODULE.validate_requirements(self.project, False)

    def test_selected_project_is_isolated_from_other_projects(self) -> None:
        self.valid_project()
        other = (
            self.root
            / "docs"
            / "smartpowers"
            / "projects"
            / "20260908-190000-other-work"
        )
        other.mkdir()
        (other / "PROJECT.md").write_text("broken", encoding="utf-8")

        selected = MODULE.resolve_project(self.root, self.project_id)
        requirements = MODULE.validate_requirements(selected, True)
        MODULE.validate_design(selected, requirements, True)
        MODULE.validate_plan(selected, requirements)


if __name__ == "__main__":
    unittest.main()
