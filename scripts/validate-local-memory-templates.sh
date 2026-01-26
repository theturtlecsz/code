#!/bin/bash
# validate-local-memory-templates.sh
# Validates that templates include mandatory local-memory section and no MCP mentions
#
# Usage: ./scripts/validate-local-memory-templates.sh
# Exit codes: 0 = pass, 1 = fail

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
EXPORT_DIR="$HOME/local-memory-configs-export"

ERRORS=()

# Template files to check (repo)
REPO_TEMPLATES=(
    "$REPO_ROOT/templates/AGENTS-template.md"
    "$REPO_ROOT/templates/CLAUDE-template.md"
    "$REPO_ROOT/templates/GEMINI-template.md"
)

# Template files to check (export)
EXPORT_TEMPLATES=(
    "$EXPORT_DIR/templates/AGENTS-template.md"
    "$EXPORT_DIR/templates/CLAUDE-template.md"
    "$EXPORT_DIR/templates/GEMINI-template.md"
)

# Project scaffolding source
SCAFFOLDING_SOURCE="$REPO_ROOT/codex-rs/tui/src/chatwidget/spec_kit/project_native.rs"

echo "=== Local Memory Template Validation ==="
echo ""

# Check 1: Mandatory header in repo templates
echo "[1/5] Checking repo templates for mandatory header..."
for template in "${REPO_TEMPLATES[@]}"; do
    if [[ -f "$template" ]]; then
        if ! grep -q "## Local Memory Integration (MANDATORY)" "$template"; then
            ERRORS+=("MISSING MANDATORY header in: $template")
        else
            echo "  OK: $(basename "$template")"
        fi
    else
        ERRORS+=("FILE NOT FOUND: $template")
    fi
done

# Check 2: Mandatory header in export templates
echo "[2/5] Checking export templates for mandatory header..."
for template in "${EXPORT_TEMPLATES[@]}"; do
    if [[ -f "$template" ]]; then
        if ! grep -q "## Local Memory Integration (MANDATORY)" "$template"; then
            ERRORS+=("MISSING MANDATORY header in: $template")
        else
            echo "  OK: $(basename "$template")"
        fi
    else
        echo "  SKIP: $(basename "$template") (export dir may not exist)"
    fi
done

# Check 3: CLI+REST policy statement
echo "[3/5] Checking for CLI+REST policy statement..."
for template in "${REPO_TEMPLATES[@]}"; do
    if [[ -f "$template" ]]; then
        if ! grep -q "CLI + REST only" "$template"; then
            ERRORS+=("MISSING CLI+REST policy in: $template")
        else
            echo "  OK: $(basename "$template")"
        fi
    fi
done

# Check 4: No MCP mentions for local-memory
echo "[4/5] Checking for prohibited MCP mentions..."
for template in "${REPO_TEMPLATES[@]}"; do
    if [[ -f "$template" ]]; then
        # Check for MCP in context of local-memory (allowing "No MCP" which is correct)
        # Fail if we find patterns like "local-memory MCP" or "MCP server" near local-memory
        if grep -i "local.memory.*mcp.*server\|mcp.*local.memory" "$template" | grep -v "No MCP" >/dev/null 2>&1; then
            ERRORS+=("PROHIBITED MCP integration found in: $template")
        else
            echo "  OK: $(basename "$template")"
        fi
    fi
done

# Check 5: Project scaffolding has mandatory header
echo "[5/5] Checking project scaffolding source..."
if [[ -f "$SCAFFOLDING_SOURCE" ]]; then
    if ! grep -q "Local Memory Integration (MANDATORY)" "$SCAFFOLDING_SOURCE"; then
        ERRORS+=("MISSING MANDATORY header in scaffolding: $SCAFFOLDING_SOURCE")
    else
        echo "  OK: project_native.rs"
    fi
else
    ERRORS+=("FILE NOT FOUND: $SCAFFOLDING_SOURCE")
fi

echo ""

# Report results
if [[ ${#ERRORS[@]} -gt 0 ]]; then
    echo "=== VALIDATION FAILED ==="
    echo ""
    for error in "${ERRORS[@]}"; do
        echo "  ERROR: $error"
    done
    echo ""
    exit 1
else
    echo "=== VALIDATION PASSED ==="
    echo "All templates include mandatory local-memory section and no prohibited MCP mentions."
    exit 0
fi
