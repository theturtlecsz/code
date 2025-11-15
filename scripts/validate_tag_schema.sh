#!/bin/bash
# Validate tag schema compliance (SPEC-941)
#
# Tag Schema Policy (MEMORY-POLICY.md lines 139-186):
# - Namespaced tags: spec:, type:, component:, stage:, agent:, project:
# - General tags: curated list (~30-50 max)
# - Forbidden: date tags (2025-10-20), task IDs (t84, T12), status values (in-progress, done)
#
# See: codex-rs/MEMORY-POLICY.md#tag-schema

set -e

echo "🔍 Checking tag schema compliance..."
echo ""

VIOLATIONS=0

# Check 1: No date tags (YYYY-MM-DD format)
echo "  → Checking for date tags..."
DATE_TAGS=$(grep -rn "tags.*\(2025-\|2024-\|2023-\|2022-\)" \
    codex-rs/tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    | grep -v "^//" \
    | grep -v "// Example" \
    || true)

if [ -n "$DATE_TAGS" ]; then
    echo "❌ FAILED: Date tags detected (forbidden)"
    echo ""
    echo "$DATE_TAGS"
    echo ""
    echo "Rule: No date tags (not useful for retrieval, proliferate over time)"
    echo "Fix: Use date range filters in search queries instead"
    echo "See: codex-rs/MEMORY-POLICY.md#forbidden-tags (lines 171-186)"
    VIOLATIONS=$((VIOLATIONS + 1))
fi

# Check 2: No task ID tags (t84, T12, t21 format)
echo "  → Checking for task ID tags..."
TASK_TAGS=$(grep -rn "tags.*\([\"']t[0-9]\+[\"']\|[\"']T[0-9]\+[\"']\)" \
    codex-rs/tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    | grep -v "^//" \
    | grep -v "// Example" \
    || true)

if [ -n "$TASK_TAGS" ]; then
    echo "❌ FAILED: Task ID tags detected (forbidden)"
    echo ""
    echo "$TASK_TAGS"
    echo ""
    echo "Rule: No task ID tags (ephemeral, not useful long-term)"
    echo "Fix: Use spec: namespace instead (e.g., spec:SPEC-KIT-071)"
    echo "See: codex-rs/MEMORY-POLICY.md#forbidden-tags (lines 171-186)"
    VIOLATIONS=$((VIOLATIONS + 1))
fi

# Check 3: No status value tags (in-progress, done, blocked, complete)
echo "  → Checking for status value tags..."
STATUS_TAGS=$(grep -rn "tags.*\(in-progress\|blocked\|done\|complete\|resolved\)" \
    codex-rs/tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    | grep -v "^//" \
    | grep -v "// Example" \
    || true)

if [ -n "$STATUS_TAGS" ]; then
    echo "⚠️  WARNING: Status value tags detected (discouraged)"
    echo ""
    echo "$STATUS_TAGS"
    echo ""
    echo "Rule: Status values change over time, use search filters instead"
    echo "Fix: Remove status tags, query by date ranges or other stable attributes"
fi

# Check 4: Encourage namespaced tags
echo "  → Checking for namespaced tags..."
NAMESPACED=$(grep -rn "tags.*\(spec:\|type:\|component:\|stage:\|agent:\|project:\)" \
    codex-rs/tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    | grep -v "^//" \
    | wc -l)

echo "   Found $NAMESPACED namespaced tags (encouraged)"

# Summary
echo ""
if [ $VIOLATIONS -eq 0 ]; then
    echo "✅ PASSED: Tag schema compliance validated"
    echo "   - No date tags ✓"
    echo "   - No task ID tags ✓"
    echo "   - $NAMESPACED namespaced tags found"
    exit 0
else
    echo "❌ FAILED: $VIOLATIONS tag schema violations"
    echo ""
    echo "Tag Schema Rules:"
    echo "  - ✅ Namespaced tags (spec:, type:, component:)"
    echo "  - ❌ No date tags (2025-10-20, 2024-12-31)"
    echo "  - ❌ No task ID tags (t84, T12, t21)"
    echo "  - ❌ No status tags (in-progress, done, blocked)"
    echo ""
    echo "See: codex-rs/MEMORY-POLICY.md#tag-schema (lines 139-186)"
    exit 1
fi
