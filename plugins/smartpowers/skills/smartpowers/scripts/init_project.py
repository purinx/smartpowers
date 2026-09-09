#!/usr/bin/env python3
"""Create an isolated timestamped Smartpowers project workspace."""

from __future__ import annotations

import argparse
import re
import unicodedata
from datetime import datetime
from pathlib import Path
from typing import Optional


PROJECTS_PATH = Path("docs/smartpowers/projects")
SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


class ProjectInitError(Exception):
    pass


def slugify(value: str) -> str:
    normalized = unicodedata.normalize("NFKD", value)
    ascii_value = normalized.encode("ascii", "ignore").decode("ascii").lower()
    return re.sub(r"^-+|-+$", "", re.sub(r"[^a-z0-9]+", "-", ascii_value))


def validate_slug(slug: str) -> None:
    if len(slug) > 64 or not SLUG.fullmatch(slug):
        raise ProjectInitError(
            "Slug must be 1-64 characters of lowercase ASCII letters, digits, and single hyphens"
        )


def create_project(
    repo_root: Path,
    title: str,
    slug: Optional[str] = None,
    now: Optional[datetime] = None,
) -> Path:
    title = title.strip()
    if not title or "\n" in title or "\r" in title:
        raise ProjectInitError("Title must be a non-empty single line")

    resolved_slug = slug.strip() if slug else slugify(title)
    if not resolved_slug:
        raise ProjectInitError(
            "Could not derive an ASCII slug from the title; pass --slug explicitly"
        )
    validate_slug(resolved_slug)

    created_at = now or datetime.now().astimezone()
    project_id = f"{created_at.strftime('%Y%m%d-%H%M%S')}-{resolved_slug}"
    project_root = repo_root.resolve() / PROJECTS_PATH / project_id
    try:
        project_root.mkdir(parents=True, exist_ok=False)
    except FileExistsError as error:
        raise ProjectInitError(f"Project already exists: {project_root}") from error

    metadata = (
        f"# {title}\n\n"
        f"Project: {project_id}\n"
        "Status: Active\n"
        f"Created: {created_at.isoformat(timespec='seconds')}\n"
        f"Slug: {resolved_slug}\n"
    )
    (project_root / "PROJECT.md").write_text(metadata, encoding="utf-8")
    return project_root


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Create docs/smartpowers/projects/<timestamp>-<slug>."
    )
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--title", required=True)
    parser.add_argument("--slug")
    args = parser.parse_args()

    try:
        project_root = create_project(args.root, args.title, args.slug)
    except ProjectInitError as error:
        parser.error(str(error))

    print(f"Project: {project_root.name}")
    print(f"Path: {project_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
