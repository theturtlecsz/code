#!/usr/bin/env python3
"""
Codex-RS Architect CLI - Wrapper for NotebookLM research operations.

This script provides project-specific research capabilities using the
Codex-RS Architect notebook and custom templates.

Usage:
    ./scripts/architect_cli.py ask "Where should I add a new slash command?"
    ./scripts/architect_cli.py audit sqlx
    ./scripts/architect_cli.py risk chatwidget.rs
    ./scripts/architect_cli.py arch "event handling patterns"
    ./scripts/architect_cli.py tui "split pane layouts"
"""

import subprocess
import sys
import json
import os
from pathlib import Path

# Configuration
NOTEBOOKLM_CLI = Path.home() / "notebooklm-mcp" / "dist" / "cli" / "index.js"
NOTEBOOK_ID = "codex-rs-architect"
PROJECT_ROOT = Path(__file__).parent.parent
TEMPLATE_DIR = PROJECT_ROOT / ".notebooklm" / "templates"

# Ensure templates are in the search path
os.environ.setdefault("NOTEBOOKLM_TEMPLATE_DIR", str(TEMPLATE_DIR))


def run_cli(args: list[str], json_output: bool = False) -> str | dict:
    """Run the NotebookLM CLI with given arguments."""
    cmd = ["node", str(NOTEBOOKLM_CLI)] + args

    if json_output:
        cmd.append("--json")

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=True,
            cwd=PROJECT_ROOT
        )
        if json_output:
            return json.loads(result.stdout)
        return result.stdout
    except subprocess.CalledProcessError as e:
        print(f"CLI Error: {e.stderr}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError:
        print(f"JSON Parse Error: {result.stdout}", file=sys.stderr)
        sys.exit(1)


def ask_architect(question: str) -> None:
    """Ask a question to the Codex-RS Architect notebook."""
    print(f"Consulting Architect: {question}")
    print("-" * 60)
    output = run_cli([
        "ask",
        "-n", NOTEBOOK_ID,
        question
    ])
    print(output)


def audit_crate(crate_name: str) -> None:
    """Audit a Rust crate for security, maintenance, and stability."""
    print(f"Auditing crate: {crate_name}")
    print("-" * 60)
    output = run_cli([
        "deep-research",
        "-n", NOTEBOOK_ID,
        crate_name,
        "-t", "crate-audit",
        "--timeout", "180"
    ])
    print(output)


def assess_risk(target: str) -> None:
    """Assess risk of modifying a file or component."""
    print(f"Risk Assessment: {target}")
    print("-" * 60)
    output = run_cli([
        "fast-research",
        "-n", NOTEBOOK_ID,
        target,
        "-t", "risk-assessment"
    ])
    print(output)


def research_architecture(topic: str) -> None:
    """Research architectural patterns for a topic."""
    print(f"Architecture Research: {topic}")
    print("-" * 60)
    output = run_cli([
        "deep-research",
        "-n", NOTEBOOK_ID,
        topic,
        "-t", "arch-review",
        "--timeout", "180"
    ])
    print(output)


def research_tui(topic: str) -> None:
    """Research TUI patterns and ratatui best practices."""
    print(f"TUI Research: {topic}")
    print("-" * 60)
    output = run_cli([
        "fast-research",
        "-n", NOTEBOOK_ID,
        topic,
        "-t", "tui-patterns"
    ])
    print(output)


def analyze_query(query: str) -> None:
    """Analyze a query and suggest templates."""
    print(f"Query Analysis: {query}")
    print("-" * 60)
    output = run_cli([
        "analyze-query",
        query
    ])
    print(output)


def list_templates() -> None:
    """List available research templates."""
    output = run_cli(["research-templates", "list"])
    print(output)


def show_help() -> None:
    """Show usage help."""
    print(__doc__)
    print("\nCommands:")
    print("  ask <question>     Ask the Architect about the codebase")
    print("  audit <crate>      Deep audit of a Rust crate")
    print("  risk <file>        Risk assessment for modifying a file")
    print("  arch <topic>       Research architectural patterns")
    print("  tui <topic>        Research TUI patterns")
    print("  analyze <query>    Analyze query intent")
    print("  templates          List available templates")
    print()
    print(f"Notebook: {NOTEBOOK_ID}")
    print(f"Templates: {TEMPLATE_DIR}")


def main() -> None:
    if len(sys.argv) < 2:
        show_help()
        sys.exit(0)

    command = sys.argv[1].lower()
    args = " ".join(sys.argv[2:]) if len(sys.argv) > 2 else ""

    match command:
        case "ask":
            if not args:
                print("Error: Question required", file=sys.stderr)
                sys.exit(1)
            ask_architect(args)
        case "audit":
            if not args:
                print("Error: Crate name required", file=sys.stderr)
                sys.exit(1)
            audit_crate(args)
        case "risk":
            if not args:
                print("Error: Target file/component required", file=sys.stderr)
                sys.exit(1)
            assess_risk(args)
        case "arch":
            if not args:
                print("Error: Topic required", file=sys.stderr)
                sys.exit(1)
            research_architecture(args)
        case "tui":
            if not args:
                print("Error: Topic required", file=sys.stderr)
                sys.exit(1)
            research_tui(args)
        case "analyze":
            if not args:
                print("Error: Query required", file=sys.stderr)
                sys.exit(1)
            analyze_query(args)
        case "templates":
            list_templates()
        case "help" | "--help" | "-h":
            show_help()
        case _:
            print(f"Unknown command: {command}", file=sys.stderr)
            show_help()
            sys.exit(1)


if __name__ == "__main__":
    main()
