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
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / "docs" / "design_docs").mkdir(parents=True)
        (self.root / "tasks").mkdir()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write(self, relative: str, text: str) -> None:
        (self.root / relative).write_text(text, encoding="utf-8")

    def valid_project(self) -> None:
        self.write(
            "docs/PRD.md",
            "# PRD\n\nStatus: Approved\nApproved: 2026-09-08 by user\n\n"
            "REQ-001: login\nNFR-001: latency\n",
        )
        self.write(
            "docs/design_docs/FIRST_BUILD.md",
            "# Design\n\nStatus: Approved\nApproved: 2026-09-08 by user\n\n"
            "REQ-001 via auth flow\nNFR-001 via cache\n",
        )
        self.write(
            "tasks/PLAN.md",
            "# Plan\n\nStatus: Approved\nApproved: 2026-09-08 by user\n",
        )
        self.write(
            "tasks/01_auth.md",
            "# Auth\n\nStatus: Planned\nRequirements: REQ-001, NFR-001\n",
        )

    def test_valid_plan(self) -> None:
        self.valid_project()
        requirements = MODULE.validate_requirements(self.root, True)
        MODULE.validate_design(self.root, requirements, True)
        MODULE.validate_plan(self.root, requirements)

    def test_design_reports_missing_requirement(self) -> None:
        self.valid_project()
        self.write(
            "docs/design_docs/FIRST_BUILD.md",
            "# Design\n\nStatus: Draft\n\nREQ-001 via auth flow\n",
        )
        requirements = MODULE.validate_requirements(self.root, True)
        with self.assertRaisesRegex(MODULE.ValidationError, "NFR-001"):
            MODULE.validate_design(self.root, requirements, False)

    def test_plan_rejects_invalid_task_filename(self) -> None:
        self.valid_project()
        (self.root / "tasks" / "01_auth.md").unlink()
        self.write(
            "tasks/auth-task.md",
            "# Auth\n\nStatus: Planned\nRequirements: REQ-001, NFR-001\n",
        )
        requirements = MODULE.validate_requirements(self.root, True)
        with self.assertRaisesRegex(MODULE.ValidationError, "filenames"):
            MODULE.validate_plan(self.root, requirements)

    def test_approved_status_requires_approval_record(self) -> None:
        self.valid_project()
        self.write("tasks/PLAN.md", "# Plan\n\nStatus: Approved\n")
        requirements = MODULE.validate_requirements(self.root, True)
        with self.assertRaisesRegex(MODULE.ValidationError, "Approved:"):
            MODULE.validate_plan(self.root, requirements)

    def test_draft_plan_can_be_validated_before_review(self) -> None:
        self.valid_project()
        self.write("tasks/PLAN.md", "# Plan\n\nStatus: Draft\n")
        requirements = MODULE.validate_requirements(self.root, True)
        MODULE.validate_plan(self.root, requirements, require_approved=False)


if __name__ == "__main__":
    unittest.main()
