#!/bin/bash
# SPEC-KIT-964 Phase 8: Config isolation validation
#
# Validates hermetic isolation requirements:
# 1. Required instruction files exist (CLAUDE.md, AGENTS.md, GEMINI.md)
# 2. Templates resolve to project-local or embedded only (no global)
#
# See: docs/spec-kit/MULTI-AGENT-ARCHITECTURE.md

set -e

echo "Validating config isolation (SPEC-KIT-964)..."

# Required instruction files
REQUIRED_FILES=("CLAUDE.md" "AGENTS.md" "GEMINI.md")
MISSING_FILES=()

for file in "${REQUIRED_FILES[@]}"; do
    if [[ ! -f "$file" ]]; then
        MISSING_FILES+=("$file")
    fi
done

if [[ ${#MISSING_FILES[@]} -gt 0 ]]; then
    echo "  WARNING: Missing instruction files: ${MISSING_FILES[*]}"
    echo "  Run '/speckit.project' to scaffold or create manually."
    # Warning only - don't fail commit for missing files
    # Some projects may intentionally not have all three
fi

# Check that no templates reference global user config
# This ensures hermetic isolation (templates must be project-local or embedded)
if grep -r "~/.config/code/templates" templates/ 2>/dev/null; then
    echo "  ERROR: Templates contain global config references"
    echo "  Templates must resolve to project-local or embedded only"
    exit 1
fi

# Check for global template path patterns in Rust code that would IMPLEMENT config lookup
# (Excludes documentation comments that explain what is NOT checked)
# Look for actual code like: home_dir()?.join("templates"), dirs::config_dir()
VIOLATION=$(grep -rn 'home_dir().*join.*template\|config_dir().*join.*template\|\.config/code/templates' codex-rs/tui/src/templates/ 2>/dev/null | grep -v '//!' | grep -v '// ' || true)
if [[ -n "$VIOLATION" ]]; then
    echo "  ERROR: Template resolution code references global config"
    echo "  SPEC-KIT-964 requires project-local -> embedded resolution only"
    echo "  Violation: $VIOLATION"
    exit 1
fi

echo "  Config isolation validated"
exit 0
