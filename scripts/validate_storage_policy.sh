#!/bin/bash
# Validate SPEC-KIT-072 storage separation policy (SPEC-941)
#
# Policy (MEMORY-POLICY.md lines 351-375):
# - Workflow data (consensus artifacts, quality gates, telemetry) → SQLite
# - Knowledge (curated insights, patterns, decisions, bug fixes) → MCP local-memory
# - MCP storage requires importance ≥8 (prevent bloat)
#
# See: codex-rs/MEMORY-POLICY.md#separation-of-concerns

set -e

echo "🔍 Checking SPEC-KIT-072 storage policy compliance..."
echo ""

VIOLATIONS=0

# Check 1: No consensus artifacts in MCP calls (spec_kit modules only)
echo "  → Checking consensus storage to MCP..."
CONSENSUS_MCP=$(grep -rn "call_tool.*local-memory.*store_memory\|mcp_client\.store_memory\|mcp_manager\.call_tool.*store_memory" \
    codex-rs/tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    --exclude-dir=tests \
    --exclude=subagent_defaults.rs \
    | grep -v "^//" \
    | grep -v "// Knowledge storage" \
    | grep -v "// Human-curated knowledge only" \
    | grep -v "// Agent instructions" \
    | grep -Ev '^\s*(orchestrator_instructions|agent_instructions):' \
    || true)

if [ -n "$CONSENSUS_MCP" ]; then
    echo "❌ FAILED: Consensus/workflow artifacts stored to MCP (violates SPEC-KIT-072)"
    echo ""
    echo "$CONSENSUS_MCP"
    echo ""
    echo "Rule: Workflow data (consensus, quality gates) → SQLite"
    echo "      Knowledge (patterns, decisions, insights) → MCP local-memory"
    echo "Fix: Use consensus_db methods (store_artifact, store_synthesis) instead"
    echo "See: codex-rs/MEMORY-POLICY.md#separation-of-concerns (lines 351-375)"
    VIOLATIONS=$((VIOLATIONS + 1))
fi

# Check 2: Consensus artifacts go to SQLite
echo "  → Checking consensus storage to SQLite..."
SQLITE_CALLS=$(grep -rn "consensus_artifacts\|consensus_synthesis\|store_artifact\|store_synthesis" \
    codex-rs/tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    | grep -v "^//" \
    | wc -l)

if [ "$SQLITE_CALLS" -lt 5 ]; then
    echo "⚠️  WARNING: Expected ≥5 consensus storage SQLite calls, found $SQLITE_CALLS"
    echo "This may indicate missing consensus storage implementation."
    echo "Expected: store_artifact, store_synthesis, query methods"
fi

# Check 3: MCP importance threshold (≥8 for storage)
echo "  → Checking MCP importance threshold..."
LOW_IMPORTANCE=$(grep -rn "store_memory" \
    codex-rs/tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    -A 8 \
    | grep -B 8 "importance" \
    | grep -E "importance[\"']?\s*[:=]\s*[0-7]($|,|\))" \
    || true)

if [ -n "$LOW_IMPORTANCE" ]; then
    echo "❌ FAILED: MCP storage with importance <8 (violates memory bloat policy)"
    echo ""
    echo "$LOW_IMPORTANCE"
    echo ""
    echo "Rule: MCP storage requires importance ≥8 (quality over quantity)"
    echo "Fix: Increase importance to ≥8 or store to SQLite if workflow data"
    echo "See: codex-rs/MEMORY-POLICY.md#importance-calibration (lines 189-231)"
    VIOLATIONS=$((VIOLATIONS + 1))
fi

# Check 4: Verify SQLite storage methods exist
echo "  → Verifying SQLite storage infrastructure..."
if ! grep -q "store_artifact_with_stage_name" codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs 2>/dev/null; then
    echo "⚠️  WARNING: store_artifact_with_stage_name method not found in consensus_db.rs"
    echo "Storage infrastructure may be incomplete."
fi

# Summary
echo ""
if [ $VIOLATIONS -eq 0 ]; then
    echo "✅ PASSED: Storage policy compliance validated"
    echo "   - Consensus/workflow → SQLite ✓"
    echo "   - Knowledge → MCP (importance ≥8) ✓"
    echo "   - $SQLITE_CALLS consensus storage calls found"
    exit 0
else
    echo "❌ FAILED: $VIOLATIONS policy violations detected"
    echo ""
    echo "Storage Policy (SPEC-KIT-072):"
    echo "  - Workflow data (consensus, quality gates, telemetry) → SQLite"
    echo "  - Knowledge (patterns, decisions, insights) → MCP local-memory"
    echo "  - MCP storage requires importance ≥8 (prevent bloat)"
    echo ""
    echo "Documentation: codex-rs/MEMORY-POLICY.md (lines 351-375)"
    exit 1
fi
