#!/bin/bash
# Generate policy compliance dashboard (SPEC-941)
#
# Aggregates all policy validation results into a single Markdown report
#
# See: codex-rs/MEMORY-POLICY.md

set -e

echo "# Policy Compliance Dashboard"
echo ""
echo "**Generated**: $(date '+%Y-%m-%d %H:%M:%S')"
echo "**Repository**: theturtlecsz/code"
echo ""
echo "---"
echo ""

# Track overall status
OVERALL_PASS=true

# Rule 1: Storage Separation (SPEC-KIT-072)
echo "## Rule 1: Storage Separation (SPEC-KIT-072)"
echo ""
echo "**Policy**: Workflow data → SQLite, Knowledge → MCP local-memory"
echo ""

if bash scripts/validate_storage_policy.sh > /dev/null 2>&1; then
    STORAGE_STATUS="✅ PASS"
    # Get SQLite call count
    SQLITE_CALLS=$(grep -rn "consensus_artifacts\|consensus_synthesis\|store_artifact\|store_synthesis" \
        codex-rs/tui/src/chatwidget/spec_kit/ \
        --include="*.rs" | grep -v "^//" | wc -l)
    echo "**Status**: $STORAGE_STATUS"
    echo ""
    echo "**Details**:"
    echo "- Consensus/workflow → SQLite: ✓"
    echo "- MCP importance ≥8: ✓"
    echo "- SQLite storage calls: $SQLITE_CALLS"
else
    STORAGE_STATUS="❌ FAIL"
    OVERALL_PASS=false
    echo "**Status**: $STORAGE_STATUS"
    echo ""
    echo "**Details**: Run \`bash scripts/validate_storage_policy.sh\` for details"
fi

echo ""
echo "**Documentation**: [MEMORY-POLICY.md#separation-of-concerns](codex-rs/MEMORY-POLICY.md#separation-of-concerns)"
echo ""
echo "---"
echo ""

# Rule 2: Tag Schema Compliance
echo "## Rule 2: Tag Schema Compliance"
echo ""
echo "**Policy**: Namespaced tags, no dates, no task IDs, no status values"
echo ""

if bash scripts/validate_tag_schema.sh > /dev/null 2>&1; then
    TAG_STATUS="✅ PASS"
    # Get namespaced tag count
    NAMESPACED=$(grep -rn "tags.*\(spec:\|type:\|component:\|stage:\|agent:\|project:\)" \
        codex-rs/tui/src/chatwidget/spec_kit/ \
        --include="*.rs" | grep -v "^//" | wc -l)
    echo "**Status**: $TAG_STATUS"
    echo ""
    echo "**Details**:"
    echo "- No date tags: ✓"
    echo "- No task ID tags: ✓"
    echo "- No status tags: ✓"
    echo "- Namespaced tags: $NAMESPACED"
else
    TAG_STATUS="❌ FAIL"
    OVERALL_PASS=false
    echo "**Status**: $TAG_STATUS"
    echo ""
    echo "**Details**: Run \`bash scripts/validate_tag_schema.sh\` for details"
fi

echo ""
echo "**Documentation**: [MEMORY-POLICY.md#tag-schema](codex-rs/MEMORY-POLICY.md#tag-schema)"
echo ""
echo "---"
echo ""

# Rule 3: MCP Importance Threshold
echo "## Rule 3: MCP Importance Threshold (≥8)"
echo ""
echo "**Policy**: All MCP storage requires importance ≥8 (prevent bloat)"
echo ""

LOW_IMP=$(grep -rn "store_memory" \
    codex-rs/tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    --exclude=subagent_defaults.rs \
    -A 8 \
    | grep -B 8 "importance" \
    | grep -E "importance[\"']?\s*[:=]\s*[0-7]($|,|\))" \
    || true)

if [ -z "$LOW_IMP" ]; then
    IMP_STATUS="✅ PASS"
    echo "**Status**: $IMP_STATUS"
    echo ""
    echo "**Details**: All MCP storage calls use importance ≥8"
else
    IMP_STATUS="❌ FAIL"
    OVERALL_PASS=false
    echo "**Status**: $IMP_STATUS"
    echo ""
    echo "**Details**: Found MCP storage with importance <8"
    echo "\`\`\`"
    echo "$LOW_IMP"
    echo "\`\`\`"
fi

echo ""
echo "**Documentation**: [MEMORY-POLICY.md#importance-calibration](codex-rs/MEMORY-POLICY.md#importance-calibration)"
echo ""
echo "---"
echo ""

# Summary
echo "## Summary"
echo ""
echo "| Rule | Status | Details |"
echo "|------|--------|---------|"
echo "| Storage Separation | $STORAGE_STATUS | Workflow → SQLite, Knowledge → MCP |"
echo "| Tag Schema | $TAG_STATUS | Namespaced, no dates/task IDs |"
echo "| MCP Importance | $IMP_STATUS | Threshold ≥8 for all storage |"
echo ""

if [ "$OVERALL_PASS" = true ]; then
    echo "**Overall Status**: ✅ **ALL POLICIES COMPLIANT**"
    echo ""
    echo "All policy checks passing. Repository follows SPEC-KIT-072 storage separation and memory hygiene guidelines."
    exit 0
else
    echo "**Overall Status**: ❌ **POLICY VIOLATIONS DETECTED**"
    echo ""
    echo "Run individual validation scripts for detailed error messages and fix suggestions:"
    echo "- \`bash scripts/validate_storage_policy.sh\`"
    echo "- \`bash scripts/validate_tag_schema.sh\`"
    exit 1
fi
