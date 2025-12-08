#!/usr/bin/env python3
"""
Forensic Churn & Coupling Analysis
Analyzes git history to identify:
- Churn hotspots (files with most commits)
- Logical coupling (files that change together)
"""

import subprocess
import re
from collections import defaultdict
from datetime import datetime, timedelta
from pathlib import Path

def run_git(args: list[str]) -> str:
    """Run a git command and return output."""
    result = subprocess.run(
        ["git"] + args,
        capture_output=True,
        text=True,
        cwd=Path(__file__).parent.parent
    )
    return result.stdout

def get_commits_last_12_months() -> list[tuple[str, list[str]]]:
    """Get all commits from the last 12 months with their changed files."""
    since_date = (datetime.now() - timedelta(days=365)).strftime("%Y-%m-%d")

    # Get commit hashes with their files
    log_output = run_git([
        "log",
        f"--since={since_date}",
        "--name-only",
        "--pretty=format:COMMIT:%H",
        "--no-merges"
    ])

    commits = []
    current_hash = None
    current_files = []

    for line in log_output.strip().split("\n"):
        if line.startswith("COMMIT:"):
            if current_hash and current_files:
                commits.append((current_hash, current_files))
            current_hash = line[7:]
            current_files = []
        elif line.strip():
            # Filter to relevant files
            if any(line.endswith(ext) for ext in [".rs", ".ts", ".py", ".md", ".toml"]):
                current_files.append(line.strip())

    if current_hash and current_files:
        commits.append((current_hash, current_files))

    return commits

def calculate_churn(commits: list[tuple[str, list[str]]]) -> dict[str, int]:
    """Calculate commit count per file."""
    churn = defaultdict(int)
    for _, files in commits:
        for f in files:
            churn[f] += 1
    return dict(sorted(churn.items(), key=lambda x: -x[1]))

def calculate_coupling(commits: list[tuple[str, list[str]]], min_cochanges: int = 5) -> list[tuple[str, str, int]]:
    """Find files that change together frequently."""
    from itertools import combinations

    cochange_count = defaultdict(int)

    for _, files in commits:
        # Only consider commits with 2-10 files (larger commits are usually bulk changes)
        if 2 <= len(files) <= 10:
            for f1, f2 in combinations(sorted(files), 2):
                cochange_count[(f1, f2)] += 1

    # Filter to significant coupling
    coupled = [
        (f1, f2, count)
        for (f1, f2), count in cochange_count.items()
        if count >= min_cochanges
    ]

    return sorted(coupled, key=lambda x: -x[2])

def generate_markdown(churn: dict[str, int], coupling: list[tuple[str, str, int]]) -> str:
    """Generate the churn_matrix.md content."""
    lines = [
        "# Forensic Churn & Coupling Analysis",
        "",
        f"_Generated: {datetime.now().isoformat()}_",
        "",
        "## Churn Hotspots (Top 30)",
        "",
        "Files with the highest number of commits in the last 12 months.",
        "High churn indicates active development, potential instability, or design issues.",
        "",
        "| Rank | File | Commits | Risk Level |",
        "|------|------|---------|------------|",
    ]

    for i, (file, count) in enumerate(list(churn.items())[:30], 1):
        if count >= 50:
            risk = "🔴 Critical"
        elif count >= 25:
            risk = "🟠 High"
        elif count >= 10:
            risk = "🟡 Medium"
        else:
            risk = "🟢 Low"
        lines.append(f"| {i} | `{file}` | {count} | {risk} |")

    lines.extend([
        "",
        "## Logical Coupling (Top 20)",
        "",
        "Files that frequently change together (>= 5 co-changes).",
        "High coupling may indicate hidden dependencies or shared responsibility.",
        "",
        "| Rank | File A | File B | Co-Changes | Coupling Strength |",
        "|------|--------|--------|------------|-------------------|",
    ])

    for i, (f1, f2, count) in enumerate(coupling[:20], 1):
        if count >= 15:
            strength = "🔴 Very Strong"
        elif count >= 10:
            strength = "🟠 Strong"
        elif count >= 7:
            strength = "🟡 Moderate"
        else:
            strength = "🟢 Weak"
        # Truncate long paths for readability
        f1_short = f1 if len(f1) < 50 else "..." + f1[-47:]
        f2_short = f2 if len(f2) < 50 else "..." + f2[-47:]
        lines.append(f"| {i} | `{f1_short}` | `{f2_short}` | {count} | {strength} |")

    lines.extend([
        "",
        "## Coupling Clusters",
        "",
        "Files grouped by their coupling relationships:",
        "",
    ])

    # Build coupling clusters
    clusters = defaultdict(set)
    for f1, f2, count in coupling[:30]:
        clusters[f1].add((f2, count))
        clusters[f2].add((f1, count))

    # Sort by number of connections
    sorted_clusters = sorted(clusters.items(), key=lambda x: -len(x[1]))[:10]

    for file, connections in sorted_clusters:
        file_short = file if len(file) < 60 else "..." + file[-57:]
        lines.append(f"### `{file_short}`")
        lines.append("")
        for conn, count in sorted(connections, key=lambda x: -x[1])[:5]:
            conn_short = conn if len(conn) < 50 else "..." + conn[-47:]
            lines.append(f"- `{conn_short}` ({count} co-changes)")
        lines.append("")

    lines.extend([
        "",
        "## Summary Statistics",
        "",
        f"- Total files analyzed: {len(churn)}",
        f"- Total coupled pairs (>= 5 co-changes): {len(coupling)}",
        f"- Highest churn: {list(churn.items())[0][1] if churn else 0} commits",
        f"- Strongest coupling: {coupling[0][2] if coupling else 0} co-changes",
        "",
        "## Risk Assessment",
        "",
        "Files appearing in BOTH high churn AND high coupling are **critical risk zones**:",
        "",
    ])

    # Find files in both categories
    high_churn_files = {f for f, c in churn.items() if c >= 20}
    high_coupling_files = {f for f1, f2, c in coupling if c >= 8 for f in [f1, f2]}
    critical_files = high_churn_files & high_coupling_files

    if critical_files:
        for f in sorted(critical_files):
            churn_count = churn.get(f, 0)
            coupling_count = sum(c for f1, f2, c in coupling if f in [f1, f2])
            lines.append(f"- `{f}` (churn: {churn_count}, total coupling: {coupling_count})")
    else:
        lines.append("_No files in critical risk zone._")

    return "\n".join(lines)

def main():
    print("Analyzing git history (last 12 months)...")
    commits = get_commits_last_12_months()
    print(f"Found {len(commits)} commits")

    print("Calculating churn...")
    churn = calculate_churn(commits)
    print(f"Analyzed {len(churn)} files")

    print("Calculating logical coupling...")
    coupling = calculate_coupling(commits)
    print(f"Found {len(coupling)} coupled pairs")

    print("Generating report...")
    markdown = generate_markdown(churn, coupling)

    output_path = Path(__file__).parent / "churn_matrix.md"
    output_path.write_text(markdown)
    print(f"Written to {output_path}")

if __name__ == "__main__":
    main()
