#!/usr/bin/env python3
"""
Complexity Analysis
Estimates code complexity using LOC and indentation depth.
"""

import json
import re
from pathlib import Path
from dataclasses import dataclass, asdict

@dataclass
class FileComplexity:
    path: str
    loc: int
    sloc: int  # Source lines (non-empty, non-comment)
    max_indent: int
    avg_indent: float
    function_count: int
    complexity_score: float
    risk_level: str

def analyze_rust_file(path: Path, repo_root: Path) -> FileComplexity | None:
    """Analyze a Rust file for complexity metrics."""
    try:
        content = path.read_text(encoding="utf-8", errors="ignore")
    except Exception:
        return None

    lines = content.split("\n")
    loc = len(lines)

    # Count source lines (skip empty and comment-only lines)
    sloc = 0
    indents = []
    in_block_comment = False

    for line in lines:
        stripped = line.strip()

        # Handle block comments
        if "/*" in stripped:
            in_block_comment = True
        if "*/" in stripped:
            in_block_comment = False
            continue

        if in_block_comment:
            continue

        # Skip empty lines and single-line comments
        if not stripped or stripped.startswith("//"):
            continue

        sloc += 1

        # Calculate indentation (spaces or tabs*4)
        indent = len(line) - len(line.lstrip())
        if "\t" in line[:indent]:
            indent = line[:indent].count("\t") * 4 + line[:indent].count(" ")
        indents.append(indent // 4)  # Normalize to indent levels

    max_indent = max(indents) if indents else 0
    avg_indent = sum(indents) / len(indents) if indents else 0

    # Count functions
    fn_pattern = re.compile(r"^\s*(pub\s+)?(async\s+)?fn\s+\w+")
    function_count = len(fn_pattern.findall(content))

    # Calculate complexity score
    # Factors: SLOC, max nesting, function count
    complexity_score = (
        sloc * 0.1 +
        max_indent * 10 +
        avg_indent * 5 +
        (function_count * 0.5 if function_count > 20 else 0)
    )

    # Determine risk level
    if complexity_score >= 200:
        risk = "critical"
    elif complexity_score >= 100:
        risk = "high"
    elif complexity_score >= 50:
        risk = "medium"
    else:
        risk = "low"

    return FileComplexity(
        path=str(path.relative_to(repo_root)),
        loc=loc,
        sloc=sloc,
        max_indent=max_indent,
        avg_indent=round(avg_indent, 2),
        function_count=function_count,
        complexity_score=round(complexity_score, 1),
        risk_level=risk
    )

def analyze_typescript_file(path: Path, repo_root: Path) -> FileComplexity | None:
    """Analyze a TypeScript file for complexity metrics."""
    try:
        content = path.read_text(encoding="utf-8", errors="ignore")
    except Exception:
        return None

    lines = content.split("\n")
    loc = len(lines)

    sloc = 0
    indents = []

    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith("//") or stripped.startswith("/*") or stripped.startswith("*"):
            continue
        sloc += 1
        indent = len(line) - len(line.lstrip())
        indents.append(indent // 2)  # TS uses 2-space indents typically

    max_indent = max(indents) if indents else 0
    avg_indent = sum(indents) / len(indents) if indents else 0

    # Count functions/methods
    fn_pattern = re.compile(r"(function\s+\w+|=>\s*\{|\w+\s*\([^)]*\)\s*:\s*\w+\s*\{)")
    function_count = len(fn_pattern.findall(content))

    complexity_score = (
        sloc * 0.1 +
        max_indent * 8 +
        avg_indent * 4
    )

    if complexity_score >= 150:
        risk = "critical"
    elif complexity_score >= 75:
        risk = "high"
    elif complexity_score >= 35:
        risk = "medium"
    else:
        risk = "low"

    return FileComplexity(
        path=str(path.relative_to(repo_root)),
        loc=loc,
        sloc=sloc,
        max_indent=max_indent,
        avg_indent=round(avg_indent, 2),
        function_count=function_count,
        complexity_score=round(complexity_score, 1),
        risk_level=risk
    )

def analyze_python_file(path: Path, repo_root: Path) -> FileComplexity | None:
    """Analyze a Python file for complexity metrics."""
    try:
        content = path.read_text(encoding="utf-8", errors="ignore")
    except Exception:
        return None

    lines = content.split("\n")
    loc = len(lines)

    sloc = 0
    indents = []
    in_docstring = False

    for line in lines:
        stripped = line.strip()

        # Handle docstrings
        if '"""' in stripped or "'''" in stripped:
            count = stripped.count('"""') + stripped.count("'''")
            if count == 1:
                in_docstring = not in_docstring
            continue

        if in_docstring:
            continue

        if not stripped or stripped.startswith("#"):
            continue

        sloc += 1
        indent = len(line) - len(line.lstrip())
        indents.append(indent // 4)  # Python uses 4-space indents

    max_indent = max(indents) if indents else 0
    avg_indent = sum(indents) / len(indents) if indents else 0

    # Count functions/methods
    fn_pattern = re.compile(r"^\s*def\s+\w+", re.MULTILINE)
    function_count = len(fn_pattern.findall(content))

    complexity_score = (
        sloc * 0.1 +
        max_indent * 12 +
        avg_indent * 6
    )

    if complexity_score >= 150:
        risk = "critical"
    elif complexity_score >= 75:
        risk = "high"
    elif complexity_score >= 35:
        risk = "medium"
    else:
        risk = "low"

    return FileComplexity(
        path=str(path.relative_to(repo_root)),
        loc=loc,
        sloc=sloc,
        max_indent=max_indent,
        avg_indent=round(avg_indent, 2),
        function_count=function_count,
        complexity_score=round(complexity_score, 1),
        risk_level=risk
    )

def main():
    repo_root = Path(__file__).parent.parent
    results = []

    # Analyze Rust files
    print("Analyzing Rust files...")
    rust_files = list(repo_root.glob("codex-rs/**/*.rs"))
    for path in rust_files:
        if result := analyze_rust_file(path, repo_root):
            results.append(result)

    # Analyze TypeScript files
    print("Analyzing TypeScript files...")
    ts_files = list(repo_root.glob("**/*.ts"))
    ts_files += list(repo_root.glob("**/*.tsx"))
    for path in ts_files:
        if "node_modules" in str(path):
            continue
        if result := analyze_typescript_file(path, repo_root):
            results.append(result)

    # Analyze Python files
    print("Analyzing Python files...")
    py_files = list(repo_root.glob("**/*.py"))
    for path in py_files:
        if "venv" in str(path) or "__pycache__" in str(path):
            continue
        if result := analyze_python_file(path, repo_root):
            results.append(result)

    # Sort by complexity score
    results.sort(key=lambda x: -x.complexity_score)

    print(f"Analyzed {len(results)} files")

    # Write JSON output
    output = {
        "generated": __import__("datetime").datetime.now().isoformat(),
        "total_files": len(results),
        "by_risk": {
            "critical": len([r for r in results if r.risk_level == "critical"]),
            "high": len([r for r in results if r.risk_level == "high"]),
            "medium": len([r for r in results if r.risk_level == "medium"]),
            "low": len([r for r in results if r.risk_level == "low"]),
        },
        "files": [asdict(r) for r in results]
    }

    output_path = Path(__file__).parent / "complexity_map.json"
    output_path.write_text(json.dumps(output, indent=2))
    print(f"Written to {output_path}")

    # Print summary
    print("\nTop 10 Most Complex Files:")
    for r in results[:10]:
        print(f"  {r.risk_level.upper():8} {r.complexity_score:6.1f}  {r.path}")

if __name__ == "__main__":
    main()
