#!/usr/bin/env python3
"""
Doc audit utility (non-destructive).

Goal: make documentation cleanup repeatable by classifying markdown files using:
- Canonical doc list (KEY_DOCS + ground-truth set).
- References from canonical docs.
- References from code/scripts (string mentions).
- Markdown link graph (inbound/outbound).

This script does NOT delete anything. It prints a report and exits non-zero only
for internal errors (not for “found candidates”).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path

import signal


REPO_ROOT = Path(__file__).resolve().parents[1]


CANONICAL_DOCS = [
    "docs/KEY_DOCS.md",
    "docs/VISION.md",
    "product-requirements.md",
    "SPEC.md",
    "CLAUDE.md",
    "templates/PRD-template.md",
    "memory/constitution.md",
    "memory/local-notes.md",
]

# Docs that are not “canonical” per project policy but act as practical entrypoints.
REFERENCE_SOURCES = [
    "README.md",
    "docs/slash-commands.md",
    "docs/DEPRECATIONS.md",
    "docs/config.md",
    "docs/spec-kit/README.md",
    "docs/TUI.md",
]


DEFAULT_EXCLUDE_DIR_PREFIXES = [
    ".git/",
    ".cargo-home/",
    "target/",
    "dist/",
    "tmp/",
    "snapshots/",
    "evidence/",
    "test-logs/",
    "codex-rs/target/",
    "codex-rs/.cargo-home/",
    "codex-rs/evidence/",
    "docs/archive/",
    "docs/**/evidence/",
    "docs/**/artifacts/",
]

DEFAULT_INCLUDE_PREFIXES = [
    "docs/",
    "memory/",
    "templates/",
    "codex-rs/docs/",
]


MARKDOWN_LINK_RE = re.compile(r"(?<!\!)\[[^\]]+\]\(([^)]+)\)")


def repo_rel(path: Path) -> str:
    return path.resolve().relative_to(REPO_ROOT).as_posix()


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def is_external_link(target: str) -> bool:
    return bool(re.match(r"^[a-zA-Z][a-zA-Z0-9+.\-]*:", target)) or target.startswith("mailto:")


def should_exclude(rel: str, exclude_prefixes: list[str]) -> bool:
    for prefix in exclude_prefixes:
        if prefix.endswith("/**"):
            base = prefix[:-3]
            if rel.startswith(base):
                return True
        if "/**/" in prefix:
            # Very small glob support: "docs/**/evidence/"
            parts = prefix.split("/**/")
            if len(parts) == 2:
                left, right = parts
                if rel.startswith(left) and right in rel:
                    return True
        if rel.startswith(prefix):
            return True
    return False


def should_include(rel: str, include_prefixes: list[str]) -> bool:
    if "/" not in rel:
        # repo-root markdown files
        return True
    return any(rel.startswith(prefix) for prefix in include_prefixes)


def find_markdown_files(exclude_prefixes: list[str], include_prefixes: list[str]) -> list[Path]:
    files: list[Path] = []
    for path in REPO_ROOT.rglob("*.md"):
        rel = repo_rel(path)
        if not should_include(rel, include_prefixes):
            continue
        if should_exclude(rel, exclude_prefixes):
            continue
        files.append(path)
    return sorted(files, key=lambda p: repo_rel(p))


def parse_links(md_text: str) -> list[str]:
    links: list[str] = []
    for m in MARKDOWN_LINK_RE.finditer(md_text):
        raw = m.group(1).strip()
        if raw.startswith("<") and raw.endswith(">"):
            raw = raw[1:-1].strip()
        if not raw or is_external_link(raw):
            continue
        links.append(raw)
    return links


def resolve_link(from_path: Path, raw_target: str) -> str | None:
    # Drop anchors/query
    target = raw_target.split("#", 1)[0].strip()
    if target == "" or target.startswith("#"):
        return repo_rel(from_path)
    if target.startswith("/") or target.startswith("~"):
        return None
    resolved = (from_path.parent / target).resolve()
    try:
        return repo_rel(resolved)
    except Exception:
        return None


def run_rg(pattern: str, roots: list[str]) -> dict[str, list[str]]:
    import subprocess

    cmd = ["rg", "-n", "-S", pattern, *roots]
    proc = subprocess.run(cmd, cwd=REPO_ROOT, capture_output=True, text=True)
    stdout = proc.stdout or ""
    hits: dict[str, list[str]] = {}
    for line in stdout.splitlines():
        # file:line:...
        parts = line.split(":", 2)
        if len(parts) < 2:
            continue
        hits.setdefault(parts[0], []).append(line)
    return hits


@dataclass(frozen=True)
class DocInfo:
    path: str
    is_canonical: bool
    inbound_from_canonical: int
    inbound_total: int
    referenced_by_code_or_scripts: int
    referenced_by_entrypoints_plaintext: int


CODE_REF_TOKEN_RE = re.compile(r"[A-Za-z0-9_./-]+\.md")


def is_probably_binary(data: str) -> bool:
    return "\0" in data


def iter_code_files(code_roots: list[str]) -> list[Path]:
    # Scan only “code-like” files for doc references; avoid pulling docs into this signal.
    allowed_suffixes = {
        ".rs",
        ".py",
        ".sh",
        ".zsh",
        ".bash",
        ".toml",
        ".yml",
        ".yaml",
        ".json",
        ".js",
        ".ts",
        ".tsx",
        ".jsx",
        ".txt",
    }

    paths: list[Path] = []
    for root in code_roots:
        p = (REPO_ROOT / root).resolve()
        if not p.exists():
            continue
        if p.is_file():
            paths.append(p)
            continue
        for child in p.rglob("*"):
            if not child.is_file():
                continue
            if child.name.startswith("."):
                continue
            if child.suffix in allowed_suffixes or child.name in {"Justfile"} or child.name.endswith(".md.in"):
                paths.append(child)
    # Stable ordering for determinism.
    return sorted(set(paths), key=lambda x: x.as_posix())


def normalize_md_token(from_path: Path, raw: str) -> str | None:
    if raw.startswith(("http://", "https://", "mailto:")):
        return None
    if raw.startswith(("~", "/")):
        return None

    # Try relative resolution (./, ../) first.
    if raw.startswith(("./", "../")):
        try:
            resolved = (from_path.parent / raw).resolve()
            return repo_rel(resolved)
        except Exception:
            return None

    # Treat as repo-relative path.
    return raw.lstrip("./")


def main() -> int:
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit JSON instead of human-readable text",
    )
    parser.add_argument(
        "--include-archives",
        action="store_true",
        help="Include docs/archive/** in the scan (default: excluded).",
    )
    parser.add_argument(
        "--include-all",
        action="store_true",
        help="Scan all *.md files under the repo (may include vendored/cache dirs).",
    )
    args = parser.parse_args()

    exclude_prefixes = [] if args.include_all else DEFAULT_EXCLUDE_DIR_PREFIXES
    if args.include_archives and "docs/archive/" in exclude_prefixes:
        exclude_prefixes = [p for p in exclude_prefixes if p != "docs/archive/"]

    include_prefixes = [] if args.include_all else DEFAULT_INCLUDE_PREFIXES

    md_files = find_markdown_files(exclude_prefixes, include_prefixes)
    md_rel = [repo_rel(p) for p in md_files]
    md_set = set(md_rel)

    basename_to_paths: dict[str, list[str]] = {}
    for rel in md_rel:
        basename_to_paths.setdefault(Path(rel).name, []).append(rel)
    unique_basenames = {bn for bn, paths in basename_to_paths.items() if len(paths) == 1}

    canonical = []
    for c in CANONICAL_DOCS:
        p = (REPO_ROOT / c).resolve()
        canonical.append(repo_rel(p))
    for p in sorted((REPO_ROOT / "docs").glob("SPEC-KIT-*/PRD.md")):
        canonical.append(repo_rel(p))
    canonical_set = set(canonical)

    reference_sources = set(canonical_set)
    for p in REFERENCE_SOURCES:
        abs_path = (REPO_ROOT / p).resolve()
        if abs_path.exists():
            reference_sources.add(repo_rel(abs_path))

    # Plain-text/backtick references are common in this repo (e.g., `docs/FOO.md`).
    reference_text: dict[str, str] = {}
    for src in sorted(reference_sources):
        src_path = REPO_ROOT / src
        if src_path.exists():
            reference_text[src] = read_text(src_path)

    # Build markdown link graph
    inbound: dict[str, set[str]] = {p: set() for p in md_rel}
    outbound: dict[str, set[str]] = {p: set() for p in md_rel}
    for p in md_files:
        src = repo_rel(p)
        for raw in parse_links(read_text(p)):
            resolved = resolve_link(p, raw)
            if not resolved:
                continue
            outbound[src].add(resolved)
            if resolved in inbound:
                inbound[resolved].add(src)

    # Code/script references: look for markdown paths or filenames in code.
    # This is heuristic and intentionally conservative: it counts mentions, not “used at runtime”.
    code_roots = [
        "scripts",
        "codex-rs",
        "config",
        "templates",
        "Justfile",
        "build-fast.sh",
        "docker-compose.yml",
        "Cargo.toml",
        "config.toml.example",
    ]
    code_files = iter_code_files(code_roots)

    # Map docs → set(code files referencing it) for:
    # - full repo-relative paths (docs/foo.md)
    # - unique basenames (foo.md) to avoid basename collisions.
    code_refs: dict[str, set[str]] = {p: set() for p in md_rel}
    for code_file in code_files:
        try:
            # Keep this fast and safe: skip huge files.
            if code_file.stat().st_size > 2_000_000:
                continue
            txt = read_text(code_file)
        except Exception:
            continue
        if is_probably_binary(txt):
            continue

        tokens = set(CODE_REF_TOKEN_RE.findall(txt))
        if not tokens:
            continue
        for token in tokens:
            norm = normalize_md_token(code_file, token)
            if not norm:
                continue
            if norm in md_set:
                code_refs[norm].add(repo_rel(code_file))
                continue
            bn = Path(norm).name
            if bn in unique_basenames:
                # Map unique basenames even when not repo-qualified.
                paths = basename_to_paths.get(bn) or []
                if len(paths) == 1:
                    code_refs[paths[0]].add(repo_rel(code_file))

    infos: list[DocInfo] = []
    for p in md_rel:
        inbound_total = len(inbound.get(p, set()))
        inbound_from_canonical = len([src for src in inbound.get(p, set()) if src in reference_sources])

        # Count plain-text references from entrypoints/canonical/reference sources.
        # If basename is unique, treat a basename mention as a reference; otherwise require path mention.
        bn = Path(p).name
        requires_full_path = bn not in unique_basenames
        referenced_plain = 0
        for src, txt in reference_text.items():
            if p in txt:
                referenced_plain += 1
                continue
            if not requires_full_path and bn in txt:
                referenced_plain += 1

        referenced_by_code = len(code_refs.get(p, set()))

        infos.append(
            DocInfo(
                path=p,
                is_canonical=p in canonical_set,
                inbound_from_canonical=inbound_from_canonical,
                inbound_total=inbound_total,
                referenced_by_code_or_scripts=referenced_by_code,
                referenced_by_entrypoints_plaintext=referenced_plain,
            )
        )

    delete_candidates = [
        d
        for d in infos
        if not d.is_canonical
        and d.inbound_from_canonical == 0
        and d.referenced_by_entrypoints_plaintext == 0
        and d.referenced_by_code_or_scripts == 0
    ]

    payload = {
        "repo_root": str(REPO_ROOT),
        "counts": {
            "markdown_files_scanned": len(infos),
            "canonical_files": len([d for d in infos if d.is_canonical]),
            "delete_candidates": len(delete_candidates),
        },
        "canonical": sorted(canonical_set),
        "delete_candidates": [d.path for d in delete_candidates],
        "code_reference_files": {
            d.path: sorted(code_refs.get(d.path, set()))
            for d in infos
            if code_refs.get(d.path)
        },
        "docs": [
            {
                "path": d.path,
                "is_canonical": d.is_canonical,
                "inbound_from_canonical": d.inbound_from_canonical,
                "inbound_total": d.inbound_total,
                "referenced_by_code_or_scripts": d.referenced_by_code_or_scripts,
                "referenced_by_entrypoints_plaintext": d.referenced_by_entrypoints_plaintext,
            }
            for d in infos
        ],
    }

    try:
        if args.json:
            print(json.dumps(payload, indent=2, sort_keys=True))
            return 0
    except BrokenPipeError:
        return 0

    try:
        print(f"Markdown files scanned: {payload['counts']['markdown_files_scanned']}")
        print(f"Canonical docs:        {payload['counts']['canonical_files']}")
        print(f"Delete candidates:     {payload['counts']['delete_candidates']}")
        print("")
        if delete_candidates:
            print("Delete candidates (heuristic; verify before deletion):")
            for d in sorted(delete_candidates, key=lambda x: x.path):
                print(f"- {d.path}")
        else:
            print("No delete candidates found under current rules.")
    except BrokenPipeError:
        return 0
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
