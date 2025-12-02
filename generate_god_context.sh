#!/bin/bash
# generate_god_context.sh - God-Level Context Generation Pipeline
# Produces NotebookLM-ready artifacts for deep codebase understanding
#
# Tools used:
#   - repomix: AST-based structural packing
#   - scc: Code complexity metrics (SLOC, complexity, etc.)
#   - git-sizer: Git repository health metrics
#   - code2flow: Call graph generation (Python/JS focused)
#   - tokei: Fast line count statistics
#   - git log: Churn hotspots + logical coupling (behavioral forensics)
#
# Usage: ./generate_god_context.sh [--diet] [output_dir]
#   --diet    Generate NotebookLM-optimized output (~2M tokens vs full ~5M)

set -eo pipefail
# Note: Not using -u to avoid issues with empty arrays in bash

# Parse arguments
DIET_MODE=false
OUTPUT_DIR="notebooklm_context"

while [[ $# -gt 0 ]]; do
    case $1 in
        --diet)
            DIET_MODE=true
            shift
            ;;
        *)
            OUTPUT_DIR="$1"
            shift
            ;;
    esac
done

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
START_TIME=$(date +%s)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info()  { echo -e "${BLUE}[INFO]${NC} $*"; }
log_ok()    { echo -e "${GREEN}[OK]${NC} $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Ensure Go binaries are in PATH
export PATH="$PATH:$(go env GOPATH 2>/dev/null)/bin"

# Check prerequisites
check_tool() {
    local tool=$1
    local install_hint=$2
    if command -v "$tool" &>/dev/null; then
        log_ok "$tool found: $(command -v "$tool")"
        return 0
    else
        log_error "$tool not found. Install: $install_hint"
        return 1
    fi
}

log_info "Checking prerequisites..."
MISSING=0
check_tool repomix "npm install -g repomix" || MISSING=$((MISSING + 1))
check_tool scc "go install github.com/boyter/scc/v3@latest" || MISSING=$((MISSING + 1))
check_tool git-sizer "go install github.com/github/git-sizer@latest" || MISSING=$((MISSING + 1))
check_tool code2flow "pipx install code2flow" || MISSING=$((MISSING + 1))
check_tool tokei "(already installed on most systems)" || log_warn "tokei optional, continuing..."

if [ "$MISSING" -gt 0 ]; then
    log_error "$MISSING required tool(s) missing. Aborting."
    exit 1
fi

# Setup output directory
log_info "Setting up output directory: $OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"
cd "$REPO_ROOT"

if [ "$DIET_MODE" = true ]; then
    log_info "DIET MODE enabled - generating NotebookLM-optimized output (~2M tokens)"
fi

# Track timing
time_step() {
    local name=$1
    local start=$2
    local end=$(date +%s)
    echo "$name: $((end - start))s"
}

# 1. Structural Map (AST-based)
log_info "Generating structural map with repomix..."
STEP_START=$(date +%s)

if [ "$DIET_MODE" = true ]; then
    # Diet mode: Source code only, no tests/docs/vendor
    log_info "  → Diet: Source-only (excluding tests, docs, vendor)"
    repomix \
        --include "**/src/**/*.rs" \
        --ignore "**/*test*" \
        --ignore "**/tests/**" \
        --ignore "**/target/**" \
        --ignore "**/vendor/**" \
        --style xml \
        --output "$OUTPUT_DIR/repo_core_logic.xml" 2>/dev/null || {
            log_warn "Diet repomix failed, trying full pack..."
            repomix --style xml --output "$OUTPUT_DIR/repo_core_logic.xml" 2>/dev/null
        }
    log_ok "repo_core_logic.xml generated ($(du -h "$OUTPUT_DIR/repo_core_logic.xml" | cut -f1))"
else
    # Full mode: Everything
    if repomix --compress --style xml --output "$OUTPUT_DIR/repo_structure.xml" 2>/dev/null; then
        log_ok "repo_structure.xml generated ($(du -h "$OUTPUT_DIR/repo_structure.xml" | cut -f1))"
    else
        log_warn "repomix failed, trying without compression..."
        repomix --style xml --output "$OUTPUT_DIR/repo_structure.xml" 2>/dev/null || {
            log_error "repomix failed completely"
            echo "# Repomix failed" > "$OUTPUT_DIR/repo_structure.xml"
        }
    fi
fi
time_step "repomix" $STEP_START

# 2. Code Metrics (complexity, SLOC)
log_info "Generating code metrics with scc..."
STEP_START=$(date +%s)
scc --format json . > "$OUTPUT_DIR/code_metrics.json" 2>/dev/null || {
    log_warn "scc JSON failed, trying wide format..."
    scc --wide . > "$OUTPUT_DIR/code_metrics.txt" 2>/dev/null
}
log_ok "code_metrics.json generated ($(du -h "$OUTPUT_DIR/code_metrics.json" 2>/dev/null | cut -f1 || echo "N/A"))"
time_step "scc" $STEP_START

# 3. Git Forensics (repo health)
log_info "Generating git forensics with git-sizer..."
STEP_START=$(date +%s)
git-sizer --json > "$OUTPUT_DIR/git_forensics.json" 2>/dev/null || {
    log_warn "git-sizer failed, falling back to basic git stats..."
    {
        echo "{"
        echo "  \"commit_count\": $(git rev-list --count HEAD 2>/dev/null || echo 0),"
        echo "  \"branch_count\": $(git branch -a | wc -l),"
        echo "  \"contributor_count\": $(git shortlog -sn HEAD 2>/dev/null | wc -l),"
        echo "  \"first_commit\": \"$(git log --reverse --format=%ci 2>/dev/null | head -1)\","
        echo "  \"last_commit\": \"$(git log -1 --format=%ci 2>/dev/null)\""
        echo "}"
    } > "$OUTPUT_DIR/git_forensics.json"
}
log_ok "git_forensics.json generated"
time_step "git-sizer" $STEP_START

# 4. Churn Hotspots (behavioral forensics - what changes most)
log_info "Generating churn hotspots..."
STEP_START=$(date +%s)
{
    echo "# Churn Hotspots (Top 50 Most Changed Files)"
    echo ""
    echo "Files with high churn are bug magnets. They change frequently,"
    echo "indicating either instability, active development, or design debt."
    echo ""
    echo "| Commits | File Path |"
    echo "|---------|-----------|"
    git log --pretty=format: --name-only 2>/dev/null | grep -v '^$' | sort | uniq -c | sort -rg | head -50 | \
        awk '{print "| " $1 " | " $2 " |"}' || true
} > "$OUTPUT_DIR/churn_hotspots.md"
log_ok "churn_hotspots.md generated"
time_step "churn" $STEP_START

# 5. Logical Coupling (behavioral forensics - what changes together)
log_info "Generating logical coupling report..."
STEP_START=$(date +%s)
{
    echo "# Logical Coupling (Files Changed Together)"
    echo ""
    echo "Files frequently modified in the same commit indicate hidden dependencies."
    echo "High coupling suggests these files should be refactored together or"
    echo "their relationship made explicit in the architecture."
    echo ""
    echo "## Top Co-Change Pairs (last 500 commits, ≥3 co-changes)"
    echo ""
    git log --pretty=format:'---COMMIT---' --name-only -500 2>/dev/null | awk '
    /^---COMMIT---$/ {
        for (i in files) for (j in files) if (i < j) pairs[i","j]++
        delete files; next
    }
    /^.+$/ { files[$0]=1 }
    END { for (p in pairs) if (pairs[p] >= 3) print pairs[p] " " p }
    ' | sort -rn | head -30 | while read -r count pair; do
        f1=$(echo "$pair" | cut -d',' -f1)
        f2=$(echo "$pair" | cut -d',' -f2)
        echo "- **$count commits**: \`$f1\` ↔ \`$f2\`"
    done || true
} > "$OUTPUT_DIR/logical_coupling.md"
log_ok "logical_coupling.md generated"
time_step "coupling" $STEP_START

# 6. Call Graph (best effort - works better for Python/JS)
log_info "Generating call graph with code2flow..."
STEP_START=$(date +%s)
# Try Python files first
PYTHON_FILES=$(find . -name "*.py" -type f -not -path "./.venv/*" 2>/dev/null | head -20)
if [ -n "$PYTHON_FILES" ]; then
    # shellcheck disable=SC2086
    (code2flow $PYTHON_FILES --output "$OUTPUT_DIR/call_graph.gv" 2>/dev/null && \
        log_ok "call_graph.gv generated from Python sources") || \
        log_warn "code2flow failed on Python files"
else
    log_warn "No Python files found for call graph"
fi

# Fallback: generate basic module dependency graph for Rust
if [ ! -f "$OUTPUT_DIR/call_graph.gv" ] || [ ! -s "$OUTPUT_DIR/call_graph.gv" ]; then
    log_info "Generating Rust module dependency graph as fallback..."
    {
        echo "digraph rust_modules {"
        echo "  rankdir=LR;"
        echo "  node [shape=box];"
        # Find all mod.rs and lib.rs, extract module structure
        (find . -name "*.rs" -type f 2>/dev/null | while read -r file; do
            module=$(echo "$file" | sed 's|^\./||; s|/|::|g; s|\.rs$||; s|::mod$||; s|::lib$||')
            # Find use statements
            grep -E "^use (crate|super)::" "$file" 2>/dev/null | \
                sed 's/use //; s/;.*//' | \
                while read -r dep; do
                    echo "  \"$module\" -> \"$dep\";"
                done
        done | sort -u | head -500) || true
        echo "}"
    } > "$OUTPUT_DIR/call_graph.gv"
    log_ok "Rust module graph generated as fallback"
fi
time_step "code2flow" $STEP_START

# 7. Quick line counts with tokei
if command -v tokei &>/dev/null; then
    log_info "Generating line counts with tokei..."
    STEP_START=$(date +%s)
    tokei --output json . > "$OUTPUT_DIR/tokei_stats.json" 2>/dev/null || \
        tokei . > "$OUTPUT_DIR/tokei_stats.txt" 2>/dev/null
    log_ok "tokei_stats.json generated"
    time_step "tokei" $STEP_START
fi

# 8. Generate summary manifest
log_info "Generating manifest..."
{
    echo "# God-Level Context Manifest"
    echo ""
    echo "- **Generated**: $(date -Iseconds)"
    echo "- **Repo root**: $REPO_ROOT"
    echo "- **Git branch**: $(git branch --show-current 2>/dev/null || echo 'unknown')"
    echo "- **Git commit**: $(git rev-parse HEAD 2>/dev/null || echo 'unknown')"
    echo "- **Mode**: $([ "$DIET_MODE" = true ] && echo 'Diet (~2M tokens)' || echo 'Full (~5M tokens)')"
    echo ""
    echo "## Artifacts"
    echo ""
    for f in "$OUTPUT_DIR"/*; do
        if [ -f "$f" ]; then
            echo "- \`$(basename "$f")\`: $(du -h "$f" | cut -f1)"
        fi
    done
    echo ""
    echo "## NotebookLM Upload Order (Recommended)"
    echo ""
    echo "1. \`repo_core_logic.xml\` or \`repo_structure.xml\` - Primary source"
    echo "2. \`churn_hotspots.md\` - Bug risk analysis"
    echo "3. \`logical_coupling.md\` - Hidden dependencies"
    echo "4. \`code_metrics.json\` - Complexity metrics"
    echo "5. \`git_forensics.json\` - Repository health"
} > "$OUTPUT_DIR/MANIFEST.md"

# Final summary
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo ""
log_ok "God-Level Context generation complete!"
echo ""
echo "Duration: ${DURATION}s"
echo "Output directory: $OUTPUT_DIR/"
if [ "$DIET_MODE" = true ]; then
    echo "Mode: DIET (NotebookLM-optimized, ~2M tokens)"
else
    echo "Mode: FULL (may exceed NotebookLM limits)"
    echo ""
    log_warn "TIP: Use --diet flag for NotebookLM-compatible output"
fi
echo ""
ls -lah "$OUTPUT_DIR/"
