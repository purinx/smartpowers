#!/usr/bin/env python3
"""Validate Smartpowers artifact gates and requirement traceability."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import List, Set


REQUIREMENT_ID = re.compile(r"\b(?:REQ|NFR)-\d{3}\b")
STATUS = re.compile(r"^Status:\s*(.+?)\s*$", re.MULTILINE | re.IGNORECASE)
APPROVED = re.compile(r"^Approved:\s*.+$", re.MULTILINE | re.IGNORECASE)
TASK_NAME = re.compile(r"^(\d{2})_[a-z0-9_]+\.md$")


class ValidationError(Exception):
    pass


def read_required(path: Path) -> str:
    if not path.is_file():
        raise ValidationError(f"Missing required file: {path}")
    return path.read_text(encoding="utf-8")


def require_status(path: Path, text: str, expected: str) -> None:
    match = STATUS.search(text)
    if not match:
        raise ValidationError(f"Missing 'Status:' field in {path}")
    actual = match.group(1).strip()
    if actual.casefold() != expected.casefold():
        raise ValidationError(
            f"{path} must have Status: {expected}; found Status: {actual}"
        )
    if expected.casefold() == "approved" and not APPROVED.search(text):
        raise ValidationError(f"Missing 'Approved:' record in {path}")


def ids(text: str) -> Set[str]:
    return set(REQUIREMENT_ID.findall(text))


def require_ids(path: Path, text: str) -> Set[str]:
    found = ids(text)
    if not found:
        raise ValidationError(f"No REQ-### or NFR-### IDs found in {path}")
    return found


def require_coverage(source: Path, expected: Set[str], target: Path, text: str) -> None:
    missing = sorted(expected - ids(text))
    if missing:
        raise ValidationError(
            f"{target} does not cover IDs from {source}: {', '.join(missing)}"
        )


def validate_requirements(root: Path, require_approved: bool) -> Set[str]:
    path = root / "docs" / "PRD.md"
    text = read_required(path)
    requirements = require_ids(path, text)
    if require_approved:
        require_status(path, text, "Approved")
    elif not STATUS.search(text):
        raise ValidationError(f"Missing 'Status:' field in {path}")
    return requirements


def validate_design(root: Path, requirements: Set[str], require_approved: bool) -> None:
    source = root / "docs" / "PRD.md"
    path = root / "docs" / "design_docs" / "FIRST_BUILD.md"
    text = read_required(path)
    require_coverage(source, requirements, path, text)
    if require_approved:
        require_status(path, text, "Approved")
    elif not STATUS.search(text):
        raise ValidationError(f"Missing 'Status:' field in {path}")


def task_files(root: Path) -> List[Path]:
    task_dir = root / "tasks"
    if not task_dir.is_dir():
        raise ValidationError(f"Missing required directory: {task_dir}")
    found = sorted(path for path in task_dir.glob("*.md") if path.name != "PLAN.md")
    invalid = [path.name for path in found if not TASK_NAME.fullmatch(path.name)]
    if invalid:
        raise ValidationError(
            "Task filenames must match NN_lowercase_snake_case.md: "
            + ", ".join(invalid)
        )
    if not found:
        raise ValidationError(f"No numbered task files found in {task_dir}")
    numbers = [TASK_NAME.fullmatch(path.name).group(1) for path in found]
    if len(numbers) != len(set(numbers)):
        raise ValidationError("Task number prefixes must be unique")
    return found


def validate_plan(
    root: Path, requirements: Set[str], require_approved: bool = True
) -> None:
    plan_path = root / "tasks" / "PLAN.md"
    plan_text = read_required(plan_path)
    if require_approved:
        require_status(plan_path, plan_text, "Approved")
    elif not STATUS.search(plan_text):
        raise ValidationError(f"Missing 'Status:' field in {plan_path}")

    files = task_files(root)
    combined: List[str] = []
    for path in files:
        text = read_required(path)
        if not STATUS.search(text):
            raise ValidationError(f"Missing 'Status:' field in {path}")
        if not ids(text):
            raise ValidationError(f"No requirement IDs found in {path}")
        combined.append(text)

    require_coverage(
        root / "docs" / "PRD.md", requirements, root / "tasks", "\n".join(combined)
    )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate Smartpowers workflow artifacts before a phase transition."
    )
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument(
        "--phase",
        choices=("requirements", "design", "plan", "implementation"),
        default="plan",
    )
    args = parser.parse_args()
    root = args.root.resolve()

    try:
        if args.phase == "requirements":
            validate_requirements(root, require_approved=False)
        elif args.phase == "design":
            requirements = validate_requirements(root, require_approved=True)
            validate_design(root, requirements, require_approved=False)
        elif args.phase == "plan":
            requirements = validate_requirements(root, require_approved=True)
            validate_design(root, requirements, require_approved=True)
            validate_plan(root, requirements, require_approved=False)
        else:
            requirements = validate_requirements(root, require_approved=True)
            validate_design(root, requirements, require_approved=True)
            validate_plan(root, requirements, require_approved=True)
    except ValidationError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(f"OK: Smartpowers {args.phase} artifacts are structurally valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
