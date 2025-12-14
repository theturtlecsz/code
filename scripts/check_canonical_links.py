#!/usr/bin/env python3
"""
Validate internal Markdown links for the repo's canonical documentation set.

Checks:
- Relative path link targets exist.
- #anchor links resolve to a real heading in the target file (GitHub-style).

This script is intentionally small and dependency-free.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]

MARKDOWN_LINK_RE = re.compile(r"(?<!\!)\[[^\]]+\]\(([^)]+)\)")
HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*$")
FENCED_CODE_BLOCK_RE = re.compile(r"```[\s\S]*?```", re.MULTILINE)
INLINE_CODE_RE = re.compile(r"`[^`]*`")


def is_external_link(target: str) -> bool:
    return bool(re.match(r"^[a-zA-Z][a-zA-Z0-9+.\-]*:", target)) or target.startswith("mailto:")


def repo_rel(path: Path) -> str:
    return path.resolve().relative_to(REPO_ROOT).as_posix()


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def slugify_heading(text: str) -> str:
    # Approximate GitHub anchor generation.
    s = text.strip().lower()
    # Normalize common unicode hyphen variants.
    s = re.sub(r"[\u2010\u2011\u2012\u2013\u2014\u2212]", "-", s)
    # Keep only alphanumerics, whitespace, and hyphens.
    s = re.sub(r"[^a-z0-9\s\-]", "", s)
    s = re.sub(r"\s+", "-", s)
    s = re.sub(r"-{2,}", "-", s).strip("-")
    return s


def extract_anchors(md_text: str) -> set[str]:
    anchors: set[str] = set()
    seen_counts: dict[str, int] = {}
    for line in md_text.splitlines():
        m = HEADING_RE.match(line)
        if not m:
            continue
        heading = m.group(2).strip()
        base = slugify_heading(heading)
        if base == "":
            continue
        count = seen_counts.get(base, 0)
        if count == 0:
            anchor = base
        else:
            anchor = f"{base}-{count}"
        seen_counts[base] = count + 1
        anchors.add(anchor)
    return anchors


def parse_links(md_text: str) -> list[str]:
    # Ignore code blocks/spans to avoid false-positive “links” from e.g. regex text: `[0-9]($|,)`.
    md_text = FENCED_CODE_BLOCK_RE.sub("", md_text)
    md_text = INLINE_CODE_RE.sub("", md_text)
    links: list[str] = []
    for m in MARKDOWN_LINK_RE.finditer(md_text):
        raw = m.group(1).strip()
        if raw.startswith("<") and raw.endswith(">"):
            raw = raw[1:-1].strip()
        if not raw or is_external_link(raw):
            continue
        links.append(raw)
    return links


@dataclass(frozen=True)
class LinkError:
    source: str
    target: str
    message: str


def resolve_target(source_path: Path, raw: str) -> tuple[Path | None, str | None]:
    # Returns (target_path, anchor) where target_path is None for non-file targets.
    if raw.startswith("#"):
        return source_path, raw[1:]
    path_part, anchor = (raw.split("#", 1) + [""])[:2]
    path_part = path_part.strip()
    if path_part == "":
        return source_path, anchor or None
    if path_part.startswith(("~", "/")):
        return None, None
    try:
        target_path = (source_path.parent / path_part).resolve()
    except Exception:
        return None, None
    return target_path, anchor or None


def canonical_paths() -> list[Path]:
    base = [
        "memory/constitution.md",
        "docs/VISION.md",
        "product-requirements.md",
        "SPEC.md",
        "CLAUDE.md",
        "templates/PRD-template.md",
        "memory/local-notes.md",
        "docs/KEY_DOCS.md",
    ]
    prds = sorted((REPO_ROOT / "docs").glob("SPEC-KIT-*/PRD.md"))
    paths = [REPO_ROOT / p for p in base] + prds
    return [p for p in paths if p.exists()]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "paths",
        nargs="*",
        help="Optional list of markdown files to check (defaults to canonical docs).",
    )
    args = parser.parse_args()

    if args.paths:
        md_paths = [(REPO_ROOT / p).resolve() for p in args.paths]
    else:
        md_paths = canonical_paths()

    anchor_cache: dict[str, set[str]] = {}
    errors: list[LinkError] = []

    for src in md_paths:
        try:
            text = read_text(src)
        except Exception as e:
            errors.append(LinkError(source=repo_rel(src), target="", message=f"Failed to read: {e}"))
            continue

        for raw in parse_links(text):
            target_path, anchor = resolve_target(src, raw)
            if target_path is None:
                continue
            if not target_path.exists():
                errors.append(
                    LinkError(
                        source=repo_rel(src),
                        target=raw,
                        message=f"Target does not exist: {repo_rel(target_path)}",
                    )
                )
                continue
            if anchor:
                key = repo_rel(target_path)
                anchors = anchor_cache.get(key)
                if anchors is None:
                    anchors = extract_anchors(read_text(target_path))
                    anchor_cache[key] = anchors
                if anchor not in anchors:
                    errors.append(
                        LinkError(
                            source=repo_rel(src),
                            target=raw,
                            message=f"Missing anchor '#{anchor}' in {key}",
                        )
                    )

    if errors:
        for e in errors:
            print(f"{e.source}: {e.target} -> {e.message}")
        return 1

    print(f"OK: checked {len(md_paths)} files, no broken internal links")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
