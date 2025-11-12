#!/usr/bin/env bash
# SPEC-KIT-927 Validation Script
# Tests robust JSON extraction with real multi-agent consensus run

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
LOG_DIR="$REPO_ROOT/test-logs/spec-927-$TIMESTAMP"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "================================================================"
echo "SPEC-KIT-927: JSON Extractor Validation"
echo "================================================================"
echo "Timestamp: $TIMESTAMP"
echo "Log directory: $LOG_DIR"
echo ""

mkdir -p "$LOG_DIR"

# Step 1: Build with new extractor
echo -e "${YELLOW}[1/5]${NC} Building with json_extractor.rs..."
cd "$REPO_ROOT/codex-rs"
if cargo build --package codex-tui --lib --profile dev-fast 2>&1 | tee "$LOG_DIR/build.log" | tail -10; then
    echo -e "${GREEN}✓${NC} Build successful"
else
    echo -e "${RED}✗${NC} Build failed - check $LOG_DIR/build.log"
    exit 1
fi
echo ""

# Step 2: Run json_extractor unit tests
echo -e "${YELLOW}[2/5]${NC} Running json_extractor unit tests..."
if cargo test --package codex-tui --lib json_extractor --no-fail-fast 2>&1 | tee "$LOG_DIR/unit-tests.log" | tail -20; then
    echo -e "${GREEN}✓${NC} All json_extractor tests pass"
else
    echo -e "${RED}✗${NC} Unit tests failed - check $LOG_DIR/unit-tests.log"
    exit 1
fi
echo ""

# Step 3: Check extraction statistics from existing agent results
echo -e "${YELLOW}[3/5]${NC} Analyzing existing agent results..."
AGENTS_DIR="$HOME/.code/agents"

if [ -d "$AGENTS_DIR" ]; then
    # Count recent agent results (<1 hour old)
    RECENT_COUNT=$(find "$AGENTS_DIR" -name "result.txt" -type f -mmin -60 2>/dev/null | wc -l)
    echo "Found $RECENT_COUNT recent agent results (<1 hour old)"

    # Test extraction on recent results
    EXTRACT_SUCCESS=0
    EXTRACT_FAIL=0

    find "$AGENTS_DIR" -name "result.txt" -type f -mmin -60 2>/dev/null | while read -r result_file; do
        agent_id=$(basename "$(dirname "$result_file")")

        # Use Python to test extraction (mimics Rust logic)
        if python3 -c "
import json
import re
import sys

content = open('$result_file').read()

# Try markdown fence
fence_match = re.search(r'\`\`\`json\s*(.*?)\`\`\`', content, re.DOTALL)
if fence_match:
    try:
        data = json.loads(fence_match.group(1))
        if 'stage' in data:
            print(f'✓ Markdown fence: {data.get(\"stage\", \"unknown\")}')
            sys.exit(0)
    except: pass

# Try depth tracking (find first { to matching })
if '{' in content:
    start = content.find('{')
    depth = 0
    for i, ch in enumerate(content[start:]):
        if ch == '{': depth += 1
        if ch == '}':
            depth -= 1
            if depth == 0:
                try:
                    data = json.loads(content[start:start+i+1])
                    if 'stage' in data:
                        print(f'✓ Depth tracking: {data.get(\"stage\", \"unknown\")}')
                        sys.exit(0)
                except: pass
                break

print('✗ Extraction failed')
sys.exit(1)
" 2>/dev/null; then
            EXTRACT_SUCCESS=$((EXTRACT_SUCCESS + 1))
        else
            EXTRACT_FAIL=$((EXTRACT_FAIL + 1))
        fi
    done

    if [ -f "$LOG_DIR/extraction-stats.txt" ]; then
        EXTRACT_SUCCESS=$(grep -c "✓" "$LOG_DIR/extraction-stats.txt" 2>/dev/null || echo 0)
        EXTRACT_FAIL=$(grep -c "✗" "$LOG_DIR/extraction-stats.txt" 2>/dev/null || echo 0)
        TOTAL=$((EXTRACT_SUCCESS + EXTRACT_FAIL))

        if [ $TOTAL -gt 0 ]; then
            SUCCESS_RATE=$(awk "BEGIN {printf \"%.1f\", ($EXTRACT_SUCCESS / $TOTAL) * 100}")
            echo "Extraction success rate: $EXTRACT_SUCCESS/$TOTAL ($SUCCESS_RATE%)"

            if (( $(echo "$SUCCESS_RATE >= 85.0" | bc -l) )); then
                echo -e "${GREEN}✓${NC} Meets 85%+ target"
            else
                echo -e "${YELLOW}⚠${NC} Below 85% target"
            fi
        fi
    fi
else
    echo "No agents directory found - skip"
fi
echo ""

# Step 4: Run quality gate integration tests
echo -e "${YELLOW}[4/5]${NC} Running quality gate integration tests..."
if cargo test --package codex-tui --lib quality_gate --no-fail-fast 2>&1 | tee "$LOG_DIR/integration-tests.log" | grep "test result:"; then
    echo -e "${GREEN}✓${NC} Quality gate tests pass"
else
    echo -e "${RED}✗${NC} Integration tests failed - check $LOG_DIR/integration-tests.log"
    exit 1
fi
echo ""

# Step 5: Summary
echo "================================================================"
echo -e "${GREEN}VALIDATION COMPLETE${NC}"
echo "================================================================"
echo "Evidence stored in: $LOG_DIR"
echo ""
echo "Next steps:"
echo "  1. Run /speckit.plan or /speckit.auto on a test SPEC"
echo "  2. Monitor extraction logs for cascade strategy usage"
echo "  3. Check agent success rates (expect 95%+)"
echo ""
echo "Expected improvements:"
echo "  • gemini: 100% → 100% (already works)"
echo "  • claude: 67% (4/6) → 95%+ (handles prose wrappers)"
echo "  • code: 0% (0/3) → 95%+ (extracts buried JSON)"
echo ""
echo "Log files:"
echo "  - Build: $LOG_DIR/build.log"
echo "  - Unit tests: $LOG_DIR/unit-tests.log"
echo "  - Integration: $LOG_DIR/integration-tests.log"
echo "  - Extraction stats: $LOG_DIR/extraction-stats.txt"
