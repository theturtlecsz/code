#!/usr/bin/env python3
"""
doc_lint.py - Documentation Contract Enforcement for Spec-Kit

SPEC-KIT-972: Validates the V6 docs contract and enforces consistency.

Usage:
    python scripts/doc_lint.py [--fix] [--verbose]

Exit codes:
    0 - All checks passed
    1 - Validation errors found
    2 - Script error

Checks performed:
    1. Required files exist (SPEC.md, PROGRAM, DECISION_REGISTER)
    2. Doc precedence order documented
    3. Active specs have Decision IDs section
    4. Merge terminology is correct (curated|full, not squash|ff)
    5. Replay Truth Table exists
    6. Invariants section present
"""

import argparse
import datetime
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

# ─────────────────────────────────────────────────────────────────────────────
# Configuration
# ─────────────────────────────────────────────────────────────────────────────

REPO_ROOT = Path(__file__).parent.parent.parent  # codex-rs/scripts/ -> codex-rs/ -> repo root

REQUIRED_FILES = {
    "SPEC.md": "Root task tracking and docs contract",
    "docs/PROGRAM.md": "Active specs, dependency DAG, sequencing gates",
    "docs/DECISIONS.md": "Locked decisions register (D1-D134)",
    "docs/POLICY.md": "Consolidated policy (model, gates, evidence, testing)",
    "docs/SPEC-KIT.md": "Canonical spec-kit reference (commands, architecture)",
    "model_policy.toml": "Machine-authoritative model policy config",
}

# Canonical docs (≤9 core + navigation) - Session 12 consolidation
CANONICAL_DOCS_ROOT = {
    "docs/KEY_DOCS.md": "Canonical doc map",
    "docs/INDEX.md": "Documentation index",
    "docs/POLICY.md": "Consolidated policy",
    "docs/OPERATIONS.md": "Consolidated operations",
    "docs/ARCHITECTURE.md": "System architecture",
    "docs/CONTRIBUTING.md": "Development workflow, fork management",
    "docs/STAGE0-REFERENCE.md": "Stage 0 engine reference",
    "docs/DECISIONS.md": "Locked decisions register",
    "docs/PROGRAM.md": "Active specs and dependency DAG",
    "docs/SPEC-KIT.md": "Spec-kit reference (CLI, architecture, quality gates)",
    "docs/VISION.md": "Product identity and vision",
}

# Migration docs (temporary, must have expiry header)
# Format: {relative_path: {"expires": "YYYY-MM-DD", "owner": "session/person", "reason": "..."}}
# When adding a migration doc:
#   1. Add it here with expiry date
#   2. Add header to doc: <!-- MIGRATION_TEMP: expires=YYYY-MM-DD owner=... -->
#   3. When expired, lint fails until doc is archived/promoted/extended
MIGRATION_ALLOWLIST = {
    # Example:
    # "docs/MERGE_BUFFER.md": {
    #     "expires": "2026-02-15",
    #     "owner": "Session-13",
    #     "reason": "Temporary buffer for Stage0 consolidation"
    # },
}

# Header pattern for migration docs
MIGRATION_HEADER_PATTERN = r"<!--\s*MIGRATION_TEMP:\s*expires=(\d{4}-\d{2}-\d{2})"

# Patterns that indicate wrong merge terminology
WRONG_MERGE_TERMS = [
    r"\bsquash\s*merge\b",
    r"\bff\s*merge\b",
    r"\bfast[- ]forward\b",
    r"\brebase\s*and\s*merge\b",
]

# Correct merge terminology
CORRECT_MERGE_TERMS = ["curated", "full"]

# Required sections in SPEC.md
SPEC_REQUIRED_SECTIONS = [
    "Doc Precedence Order",
    "Invariants",
]

# Pattern for Decision IDs
DECISION_ID_PATTERN = r"D\d{1,3}"

# ─────────────────────────────────────────────────────────────────────────────
# Data structures
# ─────────────────────────────────────────────────────────────────────────────


@dataclass
class LintError:
    """A single lint error."""
    file: str
    line: Optional[int]
    message: str
    severity: str = "error"  # error, warning

    def __str__(self):
        loc = f"{self.file}"
        if self.line:
            loc += f":{self.line}"
        return f"[{self.severity.upper()}] {loc}: {self.message}"


@dataclass
class LintResult:
    """Results from linting."""
    errors: list = field(default_factory=list)
    warnings: list = field(default_factory=list)

    def add_error(self, file: str, line: Optional[int], message: str):
        self.errors.append(LintError(file, line, message, "error"))

    def add_warning(self, file: str, line: Optional[int], message: str):
        self.warnings.append(LintError(file, line, message, "warning"))

    @property
    def passed(self) -> bool:
        return len(self.errors) == 0

    def __str__(self):
        lines = []
        for e in self.errors + self.warnings:
            lines.append(str(e))
        return "\n".join(lines)


# ─────────────────────────────────────────────────────────────────────────────
# Checks
# ─────────────────────────────────────────────────────────────────────────────


def check_required_files(result: LintResult, verbose: bool = False):
    """Check that all required documentation files exist."""
    if verbose:
        print("Checking required files...")

    for rel_path, description in REQUIRED_FILES.items():
        full_path = REPO_ROOT / rel_path
        if not full_path.exists():
            result.add_error(
                rel_path,
                None,
                f"Required file missing: {description}"
            )
        elif verbose:
            print(f"  ✓ {rel_path}")


def check_spec_md_structure(result: LintResult, verbose: bool = False):
    """Check SPEC.md has required sections."""
    spec_path = REPO_ROOT / "SPEC.md"
    if not spec_path.exists():
        return  # Already reported in check_required_files

    if verbose:
        print("Checking SPEC.md structure...")

    content = spec_path.read_text()

    for section in SPEC_REQUIRED_SECTIONS:
        # Look for markdown headers containing the section name
        pattern = rf"^#+\s*.*{re.escape(section)}.*$"
        if not re.search(pattern, content, re.MULTILINE | re.IGNORECASE):
            result.add_error(
                "SPEC.md",
                None,
                f"Missing required section: '{section}'"
            )
        elif verbose:
            print(f"  ✓ Section '{section}' found")


def check_merge_terminology(result: LintResult, verbose: bool = False):
    """Check for incorrect merge terminology in all markdown files."""
    if verbose:
        print("Checking merge terminology...")

    md_files = list(REPO_ROOT.glob("**/*.md"))
    # Exclude node_modules, target, etc.
    md_files = [f for f in md_files if not any(
        x in str(f) for x in ["node_modules", "target", ".git", "vendor"]
    )]

    wrong_count = 0
    for md_file in md_files:
        try:
            content = md_file.read_text()
            rel_path = md_file.relative_to(REPO_ROOT)

            for i, line in enumerate(content.splitlines(), 1):
                for pattern in WRONG_MERGE_TERMS:
                    if re.search(pattern, line, re.IGNORECASE):
                        result.add_warning(
                            str(rel_path),
                            i,
                            f"Wrong merge terminology. Use 'curated' or 'full', not squash/ff/rebase"
                        )
                        wrong_count += 1
        except Exception:
            pass  # Skip files we can't read

    if verbose and wrong_count == 0:
        print(f"  ✓ No incorrect merge terminology in {len(md_files)} files")


def check_decision_ids_in_specs(result: LintResult, verbose: bool = False):
    """Check that active SPEC files have Decision IDs section."""
    if verbose:
        print("Checking Decision IDs in specs...")

    # Find active spec directories
    spec_dirs = list(REPO_ROOT.glob("docs/SPEC-*"))
    spec_dirs += list(REPO_ROOT.glob("docs/spec-kit/SPEC-*"))

    for spec_dir in spec_dirs:
        if not spec_dir.is_dir():
            continue

        # Look for spec.md or similar
        spec_files = list(spec_dir.glob("*.md"))
        if not spec_files:
            continue

        rel_dir = spec_dir.relative_to(REPO_ROOT)
        has_decision_ids = False

        for spec_file in spec_files:
            content = spec_file.read_text()
            # Check for Decision IDs section
            if re.search(r"^#+\s*Decision\s+IDs?", content, re.MULTILINE | re.IGNORECASE):
                has_decision_ids = True
                break
            # Also check for inline Decision ID references
            if re.search(DECISION_ID_PATTERN, content):
                has_decision_ids = True
                break

        if not has_decision_ids:
            result.add_warning(
                str(rel_dir),
                None,
                "Spec directory missing Decision IDs section or references"
            )
        elif verbose:
            print(f"  ✓ {rel_dir} has Decision IDs")


def check_replay_truth_table(result: LintResult, verbose: bool = False):
    """Check that Replay Truth Table exists somewhere in docs."""
    if verbose:
        print("Checking Replay Truth Table...")

    # Search for replay truth table in docs
    found = False

    # Check SPEC.md first (primary location)
    spec_md = REPO_ROOT / "SPEC.md"
    if spec_md.exists():
        content = spec_md.read_text()
        if re.search(r"replay\s+truth\s+table", content, re.IGNORECASE):
            found = True
            if verbose:
                print(f"  ✓ Found in SPEC.md")

    # Also check docs folder
    if not found:
        for md_file in REPO_ROOT.glob("docs/**/*.md"):
            try:
                content = md_file.read_text()
                if re.search(r"replay\s+truth\s+table", content, re.IGNORECASE):
                    found = True
                    if verbose:
                        rel_path = md_file.relative_to(REPO_ROOT)
                        print(f"  ✓ Found in {rel_path}")
                    break
            except Exception:
                pass

    # Also check HANDOFF.md
    if not found:
        handoff = REPO_ROOT / "HANDOFF.md"
        if handoff.exists():
            content = handoff.read_text()
            if re.search(r"replay\s+truth\s+table", content, re.IGNORECASE):
                found = True
                if verbose:
                    print(f"  ✓ Found in HANDOFF.md")

    if not found:
        result.add_warning(
            "docs/",
            None,
            "Replay Truth Table not found in documentation"
        )


def check_canonical_docs(result: LintResult, verbose: bool = False):
    """Check that only allowlisted docs exist in docs/ root (no sprawl)."""
    if verbose:
        print("Checking canonical docs (anti-sprawl)...")

    docs_root = REPO_ROOT / "docs"
    if not docs_root.exists():
        result.add_error("docs/", None, "docs/ directory missing")
        return

    # Get all .md files in docs/ root (not subdirectories)
    root_md_files = set(f.name for f in docs_root.glob("*.md"))
    canonical_names = set(Path(p).name for p in CANONICAL_DOCS_ROOT.keys())
    migration_names = set(Path(p).name for p in MIGRATION_ALLOWLIST.keys())
    all_allowed = canonical_names | migration_names

    # Check for unexpected files (sprawl) - ERROR, not warning
    unexpected = root_md_files - all_allowed
    if unexpected:
        result.add_error(
            "docs/",
            None,
            f"Non-allowlisted docs in root (sprawl): {', '.join(sorted(unexpected))}"
        )
    elif verbose:
        print(f"  ✓ No doc sprawl detected ({len(root_md_files)} docs in allowlist)")

    # Check canonical docs exist
    missing_canonical = canonical_names - root_md_files
    for missing in missing_canonical:
        result.add_error(
            f"docs/{missing}",
            None,
            f"Canonical doc missing: {CANONICAL_DOCS_ROOT.get(f'docs/{missing}', 'unknown')}"
        )

    if verbose and not missing_canonical:
        print(f"  ✓ All {len(canonical_names)} canonical docs present")


def check_migration_docs(result: LintResult, verbose: bool = False):
    """Check migration docs have valid headers and are not expired."""
    if verbose:
        print("Checking migration docs...")

    if not MIGRATION_ALLOWLIST:
        if verbose:
            print("  ✓ No migration docs configured")
        return

    today = datetime.date.today()

    for rel_path, meta in MIGRATION_ALLOWLIST.items():
        full_path = REPO_ROOT / rel_path

        # Check file exists
        if not full_path.exists():
            result.add_warning(
                rel_path,
                None,
                "Migration doc in allowlist but missing (can remove from MIGRATION_ALLOWLIST)"
            )
            continue

        # Check expiry
        expires_str = meta.get("expires", "")
        if expires_str:
            try:
                expires = datetime.date.fromisoformat(expires_str)
                if today > expires:
                    result.add_error(
                        rel_path,
                        None,
                        f"Migration doc EXPIRED on {expires_str}. Archive or extend expiry."
                    )
                elif verbose:
                    days_left = (expires - today).days
                    print(f"  ✓ {Path(rel_path).name} expires in {days_left} days")
            except ValueError:
                result.add_error(
                    rel_path,
                    None,
                    f"Invalid expiry date format: {expires_str} (use YYYY-MM-DD)"
                )
        else:
            result.add_error(
                rel_path,
                None,
                "Migration doc missing 'expires' in MIGRATION_ALLOWLIST"
            )

        # Check MIGRATION_TEMP header in file content
        content = full_path.read_text()
        if not re.search(MIGRATION_HEADER_PATTERN, content):
            result.add_warning(
                rel_path,
                None,
                "Migration doc missing MIGRATION_TEMP header (add: <!-- MIGRATION_TEMP: expires=YYYY-MM-DD -->)"
            )


def check_index_canonical_listing(result: LintResult, verbose: bool = False):
    """Check INDEX.md lists all canonical docs and references Decision Register."""
    if verbose:
        print("Checking INDEX.md canonical listing...")

    index_path = REPO_ROOT / "docs" / "INDEX.md"
    if not index_path.exists():
        result.add_error("docs/INDEX.md", None, "INDEX.md missing")
        return

    content = index_path.read_text()

    # Check each canonical doc is linked
    missing = []
    for doc_path in CANONICAL_DOCS_ROOT.keys():
        doc_name = Path(doc_path).name
        if doc_name not in content:
            missing.append(doc_name)

    if missing:
        result.add_error(
            "docs/INDEX.md",
            None,
            f"Canonical docs not listed: {', '.join(missing)}"
        )
    elif verbose:
        print(f"  ✓ All {len(CANONICAL_DOCS_ROOT)} canonical docs referenced in INDEX")

    # Check Decision Register reference
    if not re.search(r"DECISIONS\.md|decision\s+register", content, re.IGNORECASE):
        result.add_error(
            "docs/INDEX.md",
            None,
            "Decision Register (DECISIONS.md) not referenced"
        )
    elif verbose:
        print("  ✓ Decision Register referenced")


def check_invariants_documented(result: LintResult, verbose: bool = False):
    """Check that key invariants are documented."""
    if verbose:
        print("Checking invariants documentation...")

    # Key invariants that should be mentioned somewhere
    key_invariants = [
        r"stage0.*no.*memvid.*dependency",
        r"logical.*uri.*immutable",
        r"single[- ]writer",
        r"mv2://",
        r"memvid.*system[- ]of[- ]record",
        r"reflex.*routing\s+mode",
        r"run.*branch",
        r"replay.*determinism|offline.*replay",
    ]

    # Search in SPEC.md, HANDOFF.md, and docs/
    search_files = [REPO_ROOT / "SPEC.md", REPO_ROOT / "HANDOFF.md"]
    search_files += list(REPO_ROOT.glob("docs/**/*.md"))

    all_content = ""
    for f in search_files:
        if f.exists():
            try:
                all_content += f.read_text() + "\n"
            except Exception:
                pass

    missing = []
    for inv_pattern in key_invariants:
        if not re.search(inv_pattern, all_content, re.IGNORECASE):
            missing.append(inv_pattern)

    if missing and verbose:
        for m in missing:
            result.add_warning(
                "docs/",
                None,
                f"Key invariant not documented: {m}"
            )
    elif verbose:
        print(f"  ✓ All key invariants documented")


# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────


def check_policy_toml_schema(result: LintResult, verbose: bool = False):
    """Check that model_policy.toml has required sections."""
    if verbose:
        print("Checking model_policy.toml schema...")

    policy_path = REPO_ROOT / "model_policy.toml"
    if not policy_path.exists():
        return  # Already reported in check_required_files

    content = policy_path.read_text()

    required_sections = [
        r"\[meta\]",
        r"\[system_of_record\]",
        r"\[routing",
        r"\[capture\]",
        r"\[budgets",
        r"\[scoring\]",
    ]

    for section in required_sections:
        if not re.search(section, content):
            section_name = section.replace("\\[", "[").replace("\\]", "]")
            result.add_error(
                "model_policy.toml",
                None,
                f"Missing required section: '{section_name}'"
            )
        elif verbose:
            section_name = section.replace("\\[", "[").replace("\\]", "]")
            print(f"  ✓ Section '{section_name}' found")


def run_all_checks(verbose: bool = False) -> LintResult:
    """Run all documentation checks."""
    result = LintResult()

    check_required_files(result, verbose)
    check_canonical_docs(result, verbose)
    check_migration_docs(result, verbose)
    check_index_canonical_listing(result, verbose)
    check_spec_md_structure(result, verbose)
    check_policy_toml_schema(result, verbose)
    check_merge_terminology(result, verbose)
    check_decision_ids_in_specs(result, verbose)
    check_replay_truth_table(result, verbose)
    check_invariants_documented(result, verbose)

    return result


def main():
    parser = argparse.ArgumentParser(
        description="Documentation contract linter for Spec-Kit"
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Show detailed progress"
    )
    parser.add_argument(
        "--fix",
        action="store_true",
        help="Attempt to fix issues (not implemented)"
    )
    parser.add_argument(
        "--warn-only",
        action="store_true",
        help="Exit 0 even with errors (treat errors as warnings)"
    )

    args = parser.parse_args()

    print("=" * 60)
    print("Doc Lint - Spec-Kit Documentation Contract Validator")
    print("=" * 60)
    print()

    result = run_all_checks(args.verbose)

    print()
    print("-" * 60)

    if result.errors:
        print(f"\n{len(result.errors)} error(s):")
        for e in result.errors:
            print(f"  {e}")

    if result.warnings:
        print(f"\n{len(result.warnings)} warning(s):")
        for w in result.warnings:
            print(f"  {w}")

    print()
    if result.passed:
        print("✅ All checks passed!")
        return 0
    else:
        print("❌ Documentation contract violations found")
        if args.warn_only:
            print("(--warn-only: exiting with 0)")
            return 0
        return 1


if __name__ == "__main__":
    sys.exit(main())
