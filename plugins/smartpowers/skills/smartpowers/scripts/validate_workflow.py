#!/usr/bin/env python3
"""Validate one Smartpowers project without crossing project boundaries."""

from __future__ import annotations

import argparse
import re
import sys
from datetime import datetime
from pathlib import Path
from typing import List, Set


PROJECTS_PATH = Path("docs/smartpowers/projects")
PROJECT_ID = re.compile(r"^\d{8}-\d{6}-[a-z0-9]+(?:-[a-z0-9]+)*$")
PROJECT_FIELD = re.compile(r"^Project:\s*(.+?)\s*$", re.MULTILINE | re.IGNORECASE)
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


def require_project(path: Path, text: str, project_id: str) -> None:
    match = PROJECT_FIELD.search(text)
    if not match:
        raise ValidationError(f"Missing 'Project:' field in {path}")
    actual = match.group(1).strip()
    if actual != project_id:
        raise ValidationError(
            f"{path} belongs to project {actual}, expected {project_id}"
        )


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


def resolve_project(repo_root: Path, project_id: str) -> Path:
    if not PROJECT_ID.fullmatch(project_id):
        raise ValidationError(
            "Project ID must match YYYYMMDD-HHMMSS-lowercase-kebab-slug"
        )
    project_root = repo_root / PROJECTS_PATH / project_id
    metadata = read_required(project_root / "PROJECT.md")
    require_project(project_root / "PROJECT.md", metadata, project_id)
    return project_root


def list_projects(repo_root: Path) -> List[str]:
    projects_root = repo_root / PROJECTS_PATH
    if not projects_root.is_dir():
        return []

    rows: List[str] = []
    for project_root in sorted(path for path in projects_root.iterdir() if path.is_dir()):
        if not PROJECT_ID.fullmatch(project_root.name):
            continue
        metadata_path = project_root / "PROJECT.md"
        if not metadata_path.is_file():
            rows.append(f"{project_root.name}\tInvalid (missing PROJECT.md)")
            continue
        metadata = metadata_path.read_text(encoding="utf-8")
        status_match = STATUS.search(metadata)
        status = status_match.group(1).strip() if status_match else "Unknown"
        title = project_root.name
        first_line = metadata.splitlines()[0] if metadata.splitlines() else ""
        if first_line.startswith("# "):
            title = first_line[2:].strip()
        latest_mtime = max(
            path.stat().st_mtime for path in project_root.rglob("*") if path.is_file()
        )
        updated = datetime.fromtimestamp(latest_mtime).astimezone().isoformat(
            timespec="seconds"
        )
        rows.append(f"{project_root.name}\t{status}\t{updated}\t{title}")
    return rows


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


def validate_requirements(project_root: Path, require_approved: bool) -> Set[str]:
    path = project_root / "PRD.md"
    text = read_required(path)
    require_project(path, text, project_root.name)
    requirements = require_ids(path, text)
    if require_approved:
        require_status(path, text, "Approved")
    elif not STATUS.search(text):
        raise ValidationError(f"Missing 'Status:' field in {path}")
    return requirements


def validate_design(
    project_root: Path, requirements: Set[str], require_approved: bool
) -> None:
    source = project_root / "PRD.md"
    path = project_root / "design_docs" / "FIRST_BUILD.md"
    text = read_required(path)
    require_project(path, text, project_root.name)
    require_coverage(source, requirements, path, text)
    if require_approved:
        require_status(path, text, "Approved")
    elif not STATUS.search(text):
        raise ValidationError(f"Missing 'Status:' field in {path}")


def task_files(project_root: Path) -> List[Path]:
    task_dir = project_root / "tasks"
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
    project_root: Path, requirements: Set[str], require_approved: bool = True
) -> None:
    plan_path = project_root / "tasks" / "PLAN.md"
    plan_text = read_required(plan_path)
    require_project(plan_path, plan_text, project_root.name)
    if require_approved:
        require_status(plan_path, plan_text, "Approved")
    elif not STATUS.search(plan_text):
        raise ValidationError(f"Missing 'Status:' field in {plan_path}")

    files = task_files(project_root)
    combined: List[str] = []
    for path in files:
        text = read_required(path)
        require_project(path, text, project_root.name)
        if not STATUS.search(text):
            raise ValidationError(f"Missing 'Status:' field in {path}")
        if not ids(text):
            raise ValidationError(f"No requirement IDs found in {path}")
        combined.append(text)

    require_coverage(
        project_root / "PRD.md",
        requirements,
        project_root / "tasks",
        "\n".join(combined),
    )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate one timestamped Smartpowers project."
    )
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--project", help="Project ID under docs/smartpowers/projects")
    parser.add_argument("--list", action="store_true", help="List available projects")
    parser.add_argument(
        "--phase",
        choices=("requirements", "design", "plan", "implementation"),
        default="implementation",
    )
    args = parser.parse_args()
    repo_root = args.root.resolve()

    if args.list:
        rows = list_projects(repo_root)
        print("\n".join(rows) if rows else "No Smartpowers projects found")
        return 0
    if not args.project:
        parser.error("--project is required unless --list is used")

    try:
        project_root = resolve_project(repo_root, args.project)
        if args.phase == "requirements":
            validate_requirements(project_root, require_approved=False)
        elif args.phase == "design":
            requirements = validate_requirements(project_root, require_approved=True)
            validate_design(project_root, requirements, require_approved=False)
        elif args.phase == "plan":
            requirements = validate_requirements(project_root, require_approved=True)
            validate_design(project_root, requirements, require_approved=True)
            validate_plan(project_root, requirements, require_approved=False)
        else:
            requirements = validate_requirements(project_root, require_approved=True)
            validate_design(project_root, requirements, require_approved=True)
            validate_plan(project_root, requirements, require_approved=True)
    except ValidationError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(f"OK: {args.project} {args.phase} artifacts are structurally valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
