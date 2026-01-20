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
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

# ─────────────────────────────────────────────────────────────────────────────
# Configuration
# ─────────────────────────────────────────────────────────────────────────────

REPO_ROOT = Path(__file__).parent.parent

REQUIRED_FILES = {
    "SPEC.md": "Root task tracking and docs contract",
    "docs/PROGRAM_2026Q1_ACTIVE.md": "Active program DAG and phase gates",
    "docs/DECISION_REGISTER.md": "Locked decisions D1-D134",
    "docs/MODEL-POLICY.md": "Human-readable model policy rationale",
    "model_policy.toml": "Machine-authoritative model policy config",
}

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
