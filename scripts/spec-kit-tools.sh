#!/bin/bash
# scripts/spec-kit-tools.sh
# Master menu for all spec-kit debug/validation/audit tools

set -e

# Get script directory and repo root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

function show_menu() {
    cat <<'EOF'
╔═══════════════════════════════════════════════════════════════╗
║           Spec-Kit Debug & Validation Toolkit                 ║
╚═══════════════════════════════════════════════════════════════╝

WORKFLOW EXECUTION:
  test <SPEC>                Run full speckit.auto workflow test
  session start <command>    Start TUI session in background
  session send <command>     Send command to running session
  session logs               View session output
  session kill               Stop session

STATUS & MONITORING:
  status <SPEC>              Comprehensive workflow dashboard
  monitor <SPEC>             Cost and performance tracking

DEBUGGING:
  debug <SPEC> <stage>       Debug consensus for stage
  agents <SPEC> <stage>      Show agent outputs side-by-side

VALIDATION:
  validate <SPEC> <stage>    Validate deliverable quality
  audit <SPEC>               Audit evidence completeness

COMPARISON:
  compare <SPEC> <run1> <run2> [stage]
                             Compare two runs (before/after)

EXAMPLES:
  spec-kit-tools test SPEC-KIT-900
  spec-kit-tools status SPEC-KIT-900
  spec-kit-tools debug SPEC-KIT-900 spec-plan
  spec-kit-tools validate SPEC-KIT-900 plan
  spec-kit-tools monitor SPEC-KIT-900

QUICK SHORTCUTS:
  # Full 6-stage workflow test (/speckit.auto)
  ./scripts/spec-kit-tools.sh test SPEC-KIT-900

  # Check status
  ./scripts/spec-kit-tools.sh status SPEC-KIT-900

  # Validate deliverable quality
  ./scripts/spec-kit-tools.sh validate SPEC-KIT-900 plan

  # Manual session for individual stages
  ./scripts/spec-kit-tools.sh session start "/speckit.plan SPEC-KIT-900"
EOF
}

function cmd_test() {
    exec bash "$SCRIPT_DIR/test-speckit-auto.sh" "$@"
}

function cmd_session() {
    exec bash "$SCRIPT_DIR/tui-session.sh" "$@"
}

function cmd_status() {
    exec bash "$SCRIPT_DIR/workflow-status.sh" "$@"
}

function cmd_debug() {
    exec bash "$SCRIPT_DIR/debug-consensus.sh" "$@"
}

function cmd_validate() {
    exec bash "$SCRIPT_DIR/validate-deliverable.sh" "$@"
}

function cmd_monitor() {
    exec bash "$SCRIPT_DIR/monitor-cost.sh" "$@"
}

function cmd_audit() {
    exec bash "$SCRIPT_DIR/audit-evidence.sh" "$@"
}

function cmd_compare() {
    exec bash "$SCRIPT_DIR/compare-runs.sh" "$@"
}

function cmd_agents() {
    SPEC_ID="$1"
    STAGE="$2"
    DB_PATH="${3:-$HOME/.code/consensus_artifacts.db}"

    if [ -z "$SPEC_ID" ] || [ -z "$STAGE" ]; then
        echo "Usage: spec-kit-tools agents <SPEC-ID> <stage>"
        exit 1
    fi

    STAGE_DB="spec-$STAGE"

    echo "==================================="
    echo "Agent Outputs: $SPEC_ID / $STAGE"
    echo "==================================="
    echo ""

    for agent in gemini claude gpt_pro gpt_codex code; do
        CONTENT=$(sqlite3 "$DB_PATH" "SELECT content FROM consensus_artifacts WHERE spec_id = '$SPEC_ID' AND stage = '$STAGE_DB' AND agent_id = '$agent' ORDER BY created_at DESC LIMIT 1;" 2>/dev/null || echo "")

        if [ -n "$CONTENT" ]; then
            echo "╔═══════════════════════════════════════════════════════════════╗"
            echo "║  Agent: $agent"
            echo "╚═══════════════════════════════════════════════════════════════╝"
            echo ""
            echo "$CONTENT"
            echo ""
            echo ""
        fi
    done
}

# Main dispatcher
case "${1:-}" in
    test)
        shift
        cmd_test "$@"
        ;;
    session)
        shift
        cmd_session "$@"
        ;;
    status)
        shift
        cmd_status "$@"
        ;;
    debug)
        shift
        cmd_debug "$@"
        ;;
    validate)
        shift
        cmd_validate "$@"
        ;;
    monitor)
        shift
        cmd_monitor "$@"
        ;;
    audit)
        shift
        cmd_audit "$@"
        ;;
    compare)
        shift
        cmd_compare "$@"
        ;;
    agents)
        shift
        cmd_agents "$@"
        ;;
    help|--help|-h|"")
        show_menu
        ;;
    *)
        echo "Unknown command: $1"
        echo ""
        show_menu
        exit 1
        ;;
esac
