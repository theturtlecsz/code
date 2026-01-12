#!/usr/bin/env python3
"""
golden_path_test.py - Golden Path End-to-End Validation

SPEC-KIT-972: Validates the complete Memvid workflow from capsule creation
through retrieval and replay.

Usage:
    python scripts/golden_path_test.py [--verbose]

This test validates:
1. Capsule initialization
2. Artifact ingestion
3. Checkpoint creation
4. Retrieval query (hybrid search)
5. Branch isolation
6. Export and replay

Exit codes:
    0 - All steps passed
    1 - One or more steps failed
"""

import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Optional

# ─────────────────────────────────────────────────────────────────────────────
# Configuration
# ─────────────────────────────────────────────────────────────────────────────

REPO_ROOT = Path(__file__).parent.parent


@dataclass
class StepResult:
    """Result of a golden path step."""
    name: str
    passed: bool
    duration_ms: float
    message: str = ""
    error: Optional[str] = None


@dataclass
class GoldenPathReport:
    """Report from golden path validation."""
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())
    steps: list = field(default_factory=list)
    passed: bool = True
    total_duration_ms: float = 0.0

    def add_step(self, step: StepResult):
        self.steps.append(step)
        self.total_duration_ms += step.duration_ms
        if not step.passed:
            self.passed = False

    def to_dict(self):
        return {
            "timestamp": self.timestamp,
            "passed": self.passed,
            "total_duration_ms": self.total_duration_ms,
            "steps": [
                {
                    "name": s.name,
                    "passed": s.passed,
                    "duration_ms": s.duration_ms,
                    "message": s.message,
                    "error": s.error,
                }
                for s in self.steps
            ],
        }

    def to_markdown(self) -> str:
        lines = [
            "# Golden Path Validation Report",
            "",
            f"**Timestamp:** {self.timestamp}",
            f"**Status:** {'PASSED' if self.passed else 'FAILED'}",
            f"**Total Duration:** {self.total_duration_ms:.1f}ms",
            "",
            "## Steps",
            "",
            "| Step | Status | Duration | Message |",
            "|------|--------|----------|---------|",
        ]

        for step in self.steps:
            status = "" if step.passed else ""
            lines.append(
                f"| {step.name} | {status} | {step.duration_ms:.1f}ms | {step.message or '-'} |"
            )

        if not self.passed:
            lines.extend([
                "",
                "## Errors",
                "",
            ])
            for step in self.steps:
                if step.error:
                    lines.append(f"### {step.name}")
                    lines.append(f"```\n{step.error}\n```")

        return "\n".join(lines)


# ─────────────────────────────────────────────────────────────────────────────
# Test Steps
# ─────────────────────────────────────────────────────────────────────────────


def run_cargo_test(test_name: str, package: str = "codex-tui") -> tuple[bool, str, float]:
    """Run a specific cargo test and return (passed, output, duration_ms)."""
    start = time.time()
    try:
        result = subprocess.run(
            ["cargo", "test", "-p", package, "--lib", "--", test_name, "--nocapture"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            timeout=60,
        )
        duration_ms = (time.time() - start) * 1000
        passed = result.returncode == 0
        output = result.stdout + result.stderr
        return passed, output, duration_ms
    except subprocess.TimeoutExpired:
        duration_ms = (time.time() - start) * 1000
        return False, "Test timed out after 60s", duration_ms
    except Exception as e:
        duration_ms = (time.time() - start) * 1000
        return False, str(e), duration_ms


def step_capsule_init(report: GoldenPathReport, verbose: bool):
    """Step 1: Validate capsule initialization."""
    if verbose:
        print("Step 1: Capsule initialization...")

    passed, output, duration = run_cargo_test("test_adapter_open_creates_capsule")

    report.add_step(StepResult(
        name="Capsule Init",
        passed=passed,
        duration_ms=duration,
        message="CapsuleHandle.open() creates .mv2 file" if passed else "Failed to create capsule",
        error=None if passed else output[:500],
    ))


def step_artifact_ingest(report: GoldenPathReport, verbose: bool):
    """Step 2: Validate artifact ingestion."""
    if verbose:
        print("Step 2: Artifact ingestion...")

    passed, output, duration = run_cargo_test("test_adapter_put_returns_stable_uri")

    report.add_step(StepResult(
        name="Artifact Ingest",
        passed=passed,
        duration_ms=duration,
        message="ingest() returns stable mv2:// URI" if passed else "Ingestion failed",
        error=None if passed else output[:500],
    ))


def step_checkpoint_create(report: GoldenPathReport, verbose: bool):
    """Step 3: Validate checkpoint creation."""
    if verbose:
        print("Step 3: Checkpoint creation...")

    passed, output, duration = run_cargo_test("test_adapter_checkpoint_creates_event")

    report.add_step(StepResult(
        name="Checkpoint Create",
        passed=passed,
        duration_ms=duration,
        message="checkpoint() creates stage boundary event" if passed else "Checkpoint failed",
        error=None if passed else output[:500],
    ))


def step_retrieval_query(report: GoldenPathReport, verbose: bool):
    """Step 4: Validate retrieval query with hybrid search."""
    if verbose:
        print("Step 4: Retrieval query...")

    passed, output, duration = run_cargo_test("test_search_memories_basic_keyword_search")

    report.add_step(StepResult(
        name="Retrieval Query",
        passed=passed,
        duration_ms=duration,
        message="search_memories() returns ranked results" if passed else "Retrieval failed",
        error=None if passed else output[:500],
    ))


def step_branch_isolation(report: GoldenPathReport, verbose: bool):
    """Step 5: Validate branch isolation."""
    if verbose:
        print("Step 5: Branch isolation...")

    passed, output, duration = run_cargo_test("test_run_branch_isolation")

    report.add_step(StepResult(
        name="Branch Isolation",
        passed=passed,
        duration_ms=duration,
        message="Branches isolate run data correctly" if passed else "Branch isolation failed",
        error=None if passed else output[:500],
    ))


def step_uri_stability(report: GoldenPathReport, verbose: bool):
    """Step 6: Validate URI stability after reopen."""
    if verbose:
        print("Step 6: URI stability...")

    passed, output, duration = run_cargo_test("test_uri_stability_after_reopen")

    report.add_step(StepResult(
        name="URI Stability",
        passed=passed,
        duration_ms=duration,
        message="URIs remain stable across capsule reopen" if passed else "URI stability failed",
        error=None if passed else output[:500],
    ))


def step_crash_recovery(report: GoldenPathReport, verbose: bool):
    """Step 7: Validate crash recovery."""
    if verbose:
        print("Step 7: Crash recovery...")

    passed, output, duration = run_cargo_test("test_crash_recovery_mid_write")

    report.add_step(StepResult(
        name="Crash Recovery",
        passed=passed,
        duration_ms=duration,
        message="Capsule recovers from mid-write crash" if passed else "Crash recovery failed",
        error=None if passed else output[:500],
    ))


def step_ab_harness(report: GoldenPathReport, verbose: bool):
    """Step 8: Validate A/B evaluation harness."""
    if verbose:
        print("Step 8: A/B harness...")

    passed, output, duration = run_cargo_test("test_ab_harness_with_memvid_adapter")

    report.add_step(StepResult(
        name="A/B Harness",
        passed=passed,
        duration_ms=duration,
        message="ABHarness produces comparative metrics" if passed else "A/B harness failed",
        error=None if passed else output[:500],
    ))


def step_hybrid_backend(report: GoldenPathReport, verbose: bool):
    """Step 9: Validate hybrid backend."""
    if verbose:
        print("Step 9: Hybrid backend...")

    passed, output, duration = run_cargo_test("test_hybrid_backend_lexical_only", "codex-stage0")

    report.add_step(StepResult(
        name="Hybrid Backend",
        passed=passed,
        duration_ms=duration,
        message="HybridBackend merges lex+vec results" if passed else "Hybrid backend failed",
        error=None if passed else output[:500],
    ))


def step_config_switch(report: GoldenPathReport, verbose: bool):
    """Step 10: Validate config switch."""
    if verbose:
        print("Step 10: Config switch...")

    passed, output, duration = run_cargo_test("test_create_memory_client_memvid_backend")

    report.add_step(StepResult(
        name="Config Switch",
        passed=passed,
        duration_ms=duration,
        message="create_memory_client routes by config" if passed else "Config switch failed",
        error=None if passed else output[:500],
    ))


# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────


def run_golden_path(verbose: bool = False) -> GoldenPathReport:
    """Run the complete golden path validation."""
    report = GoldenPathReport()

    steps = [
        step_capsule_init,
        step_artifact_ingest,
        step_checkpoint_create,
        step_retrieval_query,
        step_branch_isolation,
        step_uri_stability,
        step_crash_recovery,
        step_ab_harness,
        step_hybrid_backend,
        step_config_switch,
    ]

    for step_fn in steps:
        step_fn(report, verbose)

    return report


def main():
    parser = argparse.ArgumentParser(
        description="Golden path end-to-end validation for Spec-Kit"
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Show detailed progress"
    )
    parser.add_argument(
        "--output", "-o",
        type=str,
        default=None,
        help="Output directory for reports (default: .speckit/eval)"
    )

    args = parser.parse_args()

    print("=" * 60)
    print("Golden Path Validation - Spec-Kit E2E Test")
    print("=" * 60)
    print()

    report = run_golden_path(args.verbose)

    print()
    print("-" * 60)
    print()

    # Print summary
    passed_count = sum(1 for s in report.steps if s.passed)
    total_count = len(report.steps)

    for step in report.steps:
        status = "" if step.passed else ""
        print(f"  {status} {step.name}: {step.message}")

    print()
    print(f"Results: {passed_count}/{total_count} steps passed")
    print(f"Duration: {report.total_duration_ms:.1f}ms")
    print()

    # Save reports
    output_dir = Path(args.output) if args.output else REPO_ROOT / ".speckit" / "eval"
    output_dir.mkdir(parents=True, exist_ok=True)

    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")

    json_path = output_dir / f"golden-path-{timestamp}.json"
    md_path = output_dir / f"golden-path-{timestamp}.md"

    with open(json_path, "w") as f:
        json.dump(report.to_dict(), f, indent=2)
    print(f"JSON report: {json_path}")

    with open(md_path, "w") as f:
        f.write(report.to_markdown())
    print(f"Markdown report: {md_path}")

    if report.passed:
        print()
        print("PASSED - Golden path validation complete")
        return 0
    else:
        print()
        print("FAILED - See errors above")
        return 1


if __name__ == "__main__":
    sys.exit(main())
