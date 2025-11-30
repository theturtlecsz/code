#!/bin/bash
# SPEC-KIT-964: Config Isolation Validation
#
# Validates that spec-kit agents operate in a hermetic sandbox:
# 1. No user-specific paths in prompts.json
# 2. Project instruction files exist (CLAUDE.md, AGENTS.md, GEMINI.md)
# 3. Template resolution doesn't hit global paths
# 4. Agent prompts are hermetic (no global config references)
#
# Exit codes:
# 0 - All checks pass
# 1 - Validation failures found
#
# Usage:
#   ./scripts/validate-config-isolation.sh [--verbose]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VERBOSE="${1:-}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

ERRORS=0
WARNINGS=0

log_pass() {
    echo -e "${GREEN}✓${NC} $1"
}

log_fail() {
    echo -e "${RED}✗${NC} $1"
    ((ERRORS++))
}

log_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
    ((WARNINGS++))
}

log_info() {
    if [[ "$VERBOSE" == "--verbose" ]]; then
        echo "  → $1"
    fi
}

echo "═══════════════════════════════════════════════════════════"
echo " SPEC-KIT-964: Config Isolation Validation"
echo "═══════════════════════════════════════════════════════════"
echo ""

# ============================================================
# CHECK 1: Project instruction files exist
# ============================================================
echo "▶ Check 1: Project instruction files"

REQUIRED_FILES=("CLAUDE.md" "AGENTS.md" "GEMINI.md")
for file in "${REQUIRED_FILES[@]}"; do
    if [[ -f "$REPO_ROOT/$file" ]]; then
        log_pass "$file exists"
    else
        log_fail "$file is MISSING (required for agent parity)"
    fi
done
echo ""

# ============================================================
# CHECK 2: No user-specific paths in prompts.json
# ============================================================
echo "▶ Check 2: No user-specific paths in prompts.json"

PROMPTS_FILE="$REPO_ROOT/docs/spec-kit/prompts.json"
if [[ -f "$PROMPTS_FILE" ]]; then
    # Check for hardcoded home paths
    HOME_PATHS=$(grep -E '/home/[^/]+/|/Users/[^/]+/' "$PROMPTS_FILE" 2>/dev/null || true)
    if [[ -n "$HOME_PATHS" ]]; then
        log_fail "Found hardcoded home paths in prompts.json:"
        echo "$HOME_PATHS" | head -5
    else
        log_pass "No hardcoded home paths"
    fi

    # Check for global config paths
    GLOBAL_PATHS=$(grep -E '~/\.(config|claude|gemini|code)' "$PROMPTS_FILE" 2>/dev/null || true)
    if [[ -n "$GLOBAL_PATHS" ]]; then
        # Allow description fields that explain the system
        ACTUAL_REFS=$(echo "$GLOBAL_PATHS" | grep -v '"description"' || true)
        if [[ -n "$ACTUAL_REFS" ]]; then
            log_fail "Found global config paths in prompts.json:"
            echo "$ACTUAL_REFS" | head -5
        else
            log_pass "No global config paths (description field is OK)"
        fi
    else
        log_pass "No global config paths"
    fi
else
    log_warn "prompts.json not found at $PROMPTS_FILE"
fi
echo ""

# ============================================================
# CHECK 3: Template resolution code doesn't reference global
# ============================================================
echo "▶ Check 3: Template resolution (Rust code)"

TEMPLATE_MOD="$REPO_ROOT/codex-rs/tui/src/templates/mod.rs"
if [[ -f "$TEMPLATE_MOD" ]]; then
    # Check for UserConfig enum variant (should be removed)
    if grep -q 'UserConfig' "$TEMPLATE_MOD"; then
        log_fail "templates/mod.rs still contains UserConfig variant"
    else
        log_pass "No UserConfig variant in template enum"
    fi

    # Check for config_dir usage (should be removed from resolution)
    RESOLUTION_USAGE=$(grep -n 'config_dir' "$TEMPLATE_MOD" | grep -v '//' | grep -v 'SPEC-KIT-964' || true)
    if [[ -n "$RESOLUTION_USAGE" ]]; then
        # Check if it's in the install function (OK) vs resolve functions (BAD)
        IN_RESOLVE=$(echo "$RESOLUTION_USAGE" | grep -E 'resolve_template|resolve_template_source' || true)
        if [[ -n "$IN_RESOLVE" ]]; then
            log_fail "Global config_dir still used in template resolution"
        else
            log_pass "config_dir only in install function (OK if targeting project-local)"
        fi
    else
        log_pass "No config_dir in template resolution"
    fi
else
    log_warn "templates/mod.rs not found"
fi
echo ""

# ============================================================
# CHECK 4: No global template overrides active
# ============================================================
echo "▶ Check 4: No global template overrides active"

# Check if user has global templates that could be confusing
GLOBAL_TEMPLATE_DIR="$HOME/.config/code/templates"
if [[ -d "$GLOBAL_TEMPLATE_DIR" ]]; then
    TEMPLATE_COUNT=$(ls -1 "$GLOBAL_TEMPLATE_DIR"/*.md 2>/dev/null | wc -l || echo 0)
    if [[ "$TEMPLATE_COUNT" -gt 0 ]]; then
        log_warn "Global templates exist at $GLOBAL_TEMPLATE_DIR ($TEMPLATE_COUNT files)"
        log_info "These are ignored by spec-kit (SPEC-KIT-964)"
    else
        log_pass "Global template directory is empty"
    fi
else
    log_pass "No global template directory"
fi
echo ""

# ============================================================
# CHECK 5: Project templates directory
# ============================================================
echo "▶ Check 5: Project templates"

PROJECT_TEMPLATES="$REPO_ROOT/templates"
if [[ -d "$PROJECT_TEMPLATES" ]]; then
    TEMPLATE_COUNT=$(ls -1 "$PROJECT_TEMPLATES"/*.md 2>/dev/null | wc -l || echo 0)
    log_pass "Project templates directory exists ($TEMPLATE_COUNT templates)"
else
    log_info "No project templates directory (will use embedded defaults)"
    log_pass "OK - embedded templates will be used"
fi
echo ""

# ============================================================
# CHECK 6: Instruction file parity (content similarity)
# ============================================================
echo "▶ Check 6: Instruction file parity"

if [[ -f "$REPO_ROOT/CLAUDE.md" && -f "$REPO_ROOT/GEMINI.md" ]]; then
    # Check for key sections that should exist in both
    SECTIONS=("Operating Modes" "Memory Workflow" "Config Isolation")
    for section in "${SECTIONS[@]}"; do
        CLAUDE_HAS=$(grep -c "$section" "$REPO_ROOT/CLAUDE.md" || echo 0)
        GEMINI_HAS=$(grep -c "$section" "$REPO_ROOT/GEMINI.md" || echo 0)
        if [[ "$CLAUDE_HAS" -gt 0 && "$GEMINI_HAS" -gt 0 ]]; then
            log_pass "Both have '$section' section"
        elif [[ "$CLAUDE_HAS" -gt 0 || "$GEMINI_HAS" -gt 0 ]]; then
            log_warn "Section '$section' missing from one file"
        fi
    done
else
    log_warn "Cannot check parity - files missing"
fi
echo ""

# ============================================================
# SUMMARY
# ============================================================
echo "═══════════════════════════════════════════════════════════"
if [[ $ERRORS -eq 0 ]]; then
    echo -e "${GREEN}All checks passed!${NC} ($WARNINGS warnings)"
    exit 0
else
    echo -e "${RED}$ERRORS errors found${NC} ($WARNINGS warnings)"
    echo ""
    echo "To fix:"
    echo "  1. Ensure CLAUDE.md, AGENTS.md, GEMINI.md exist in project root"
    echo "  2. Remove any hardcoded user paths from prompts.json"
    echo "  3. Verify template resolution uses hermetic order"
    echo ""
    exit 1
fi
