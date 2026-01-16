#!/usr/bin/env python3
"""
doc_lint.py - Forwarding stub to canonical location

⚠️ THIS FILE IS A FORWARDING STUB

The canonical doc_lint.py is located at:
    codex-rs/scripts/doc_lint.py

This stub forwards all arguments to the canonical version to ensure:
1. Single entrypoint for CI and pre-commit hooks
2. No drift between duplicate implementations
3. model_policy.toml checks are always enforced

See: SPEC-KIT-979 documentation consolidation
"""
import os
import subprocess
import sys
from pathlib import Path

def main():
    # Find the canonical doc_lint.py
    script_dir = Path(__file__).parent
    repo_root = script_dir.parent
    canonical = repo_root / "codex-rs" / "scripts" / "doc_lint.py"

    if not canonical.exists():
        print(f"❌ Canonical doc_lint.py not found at: {canonical}", file=sys.stderr)
        print("   Expected: codex-rs/scripts/doc_lint.py", file=sys.stderr)
        return 2

    # Forward to canonical, passing through all arguments
    # Change working directory to codex-rs so paths resolve correctly
    cwd = repo_root / "codex-rs"
    result = subprocess.run(
        [sys.executable, str(canonical)] + sys.argv[1:],
        cwd=str(cwd)
    )
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
