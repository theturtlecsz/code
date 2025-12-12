#!/usr/bin/env python3

import os
import re
import sys
from pathlib import Path


LINK_RE = re.compile(r"\[[^\]]*\]\(([^)]+)\)")

CANONICAL_MARKDOWN_FILES = [
    "README.md",
    "SPEC.md",
    "product-requirements.md",
    "CONTRIBUTING.md",
    "CLAUDE.md",
    "memory/constitution.md",
    "memory/local-notes.md",
    "templates/PRD-template.md",
    "docs/KEY_DOCS.md",
    "docs/VISION.md",
    "docs/GETTING_STARTED.md",
    "docs/CONFIG.md",
    "docs/ARCHITECTURE.md",
    "docs/DEPRECATIONS.md",
    "docs/MAINTAINER_ANSWERS.md",
    "docs/slash-commands.md",
]


def is_external(href: str) -> bool:
    return href.startswith(("http://", "https://", "mailto:"))


def iter_markdown_files(repo_root: Path):
    for rel in CANONICAL_MARKDOWN_FILES:
        path = repo_root / rel
        if path.exists():
            yield path


def check_file(path: Path, repo_root: Path) -> list[str]:
    failures: list[str] = []
    text = path.read_text(encoding="utf-8", errors="replace")

    for match in LINK_RE.finditer(text):
        href = match.group(1).strip()

        if not href or href.startswith("#") or is_external(href):
            continue

        href = href.split("#", 1)[0].split("?", 1)[0].strip()
        if not href:
            continue

        if href.startswith("/"):
            target = repo_root / href.lstrip("/")
        else:
            target = path.parent / href

        try:
            target = target.resolve()
        except FileNotFoundError:
            target = (path.parent / href).absolute()

        if not target.exists():
            failures.append(f"{path.relative_to(repo_root)}: missing link target: {href}")

    return failures


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    failures: list[str] = []

    for md in iter_markdown_files(repo_root):
        failures.extend(check_file(md, repo_root))

    if failures:
        for f in failures:
            print(f, file=sys.stderr)
        print(f"\n{len(failures)} broken local links.", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
