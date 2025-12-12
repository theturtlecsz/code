#!/usr/bin/env python3

import os
import re
import sys
from pathlib import Path


LINK_RE = re.compile(r"\[[^\]]*\]\(([^)]+)\)")


def is_external(href: str) -> bool:
    return href.startswith(("http://", "https://", "mailto:"))


def iter_markdown_files(repo_root: Path):
    candidates = []
    candidates.append(repo_root / "README.md")

    docs_root = repo_root / "docs"
    if docs_root.exists():
        for path in docs_root.rglob("*.md"):
            rel = path.relative_to(repo_root).as_posix()
            if rel.startswith("docs/archive/"):
                continue
            if "/evidence/" in rel:
                continue
            if rel.startswith("docs/SPEC-OPS-004-integrated-coder-hooks/evidence/"):
                continue
            candidates.append(path)

    for path in candidates:
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

