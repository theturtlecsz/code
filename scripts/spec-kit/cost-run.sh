#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: scripts/spec-kit/cost-run.sh [--spec SPEC-ID] [--log PATH] [--no-memory]"
}

SPEC="SPEC-KIT-070"
LOG_FILE="${HOME}/.code/log/codex-tui.log"
RUN_MEMORY=true

while [[ $# -gt 0 ]]; do
  case "$1" in
    --spec)
      SPEC="$2"
      shift 2
      ;;
    --log)
      LOG_FILE="$2"
      shift 2
      ;;
    --preset)
      PRESET="$2"
      shift 2
      ;;
    --commands)
      CUSTOM_COMMANDS="$2"
      shift 2
      ;;
    --no-memory)
      RUN_MEMORY=false
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

COST_FILE="docs/SPEC-OPS-004-integrated-coder-hooks/evidence/costs/${SPEC}_cost_summary.json"

case "$PRESET" in
  minimal)
    COMMAND_LIST=("/speckit.plan $SPEC" "/speckit.tasks $SPEC" "/speckit.validate $SPEC")
    ;;
  full)
    COMMAND_LIST=(
      "/speckit.status $SPEC"
      "/speckit.clarify $SPEC"
      "/speckit.analyze $SPEC"
      "/speckit.checklist $SPEC"
      "/speckit.plan $SPEC"
      "/speckit.tasks $SPEC"
      "/speckit.implement $SPEC"
      "/speckit.validate $SPEC"
      "/speckit.audit $SPEC"
      "/speckit.unlock $SPEC"
      "/speckit.status $SPEC"
    )
    ;;
  *)
    COMMAND_LIST=("/speckit.plan $SPEC" "/speckit.tasks $SPEC" "/speckit.validate $SPEC")
    ;;
esac

if [[ -n "$CUSTOM_COMMANDS" ]]; then
  IFS="," read -r -a COMMAND_LIST <<< "$CUSTOM_COMMANDS"
fi

if [[ ! -f "$LOG_FILE" ]]; then
  echo "Log file not found: $LOG_FILE" >&2
  exit 1
fi

echo "=== SPEC-KIT cost run helper ==="
echo "SPEC: $SPEC"
echo "Log:  $LOG_FILE"
echo "Cost summary -> $COST_FILE"
if ! $RUN_MEMORY; then
  echo "Local-memory snapshots disabled (--no-memory)."
fi
echo "Command sequence:"
for cmd in "${COMMAND_LIST[@]}"; do
  printf "  %s\n" "$cmd"
done

echo "[1/4] Removing previous cost summary (if any)."
rm -f "$COST_FILE"

STAMP=$(date -u +"%Y%m%dT%H%M%SZ")
ART_DIR="tmp/spec-kit-cost/${SPEC}_${STAMP}"
mkdir -p "$ART_DIR"
TAIL_LOG="$ART_DIR/tui_launching_agents.log"

echo "[2/4] Tailing TUI log (capturing \"Launching agent\" lines)."
echo "        Tail output -> $TAIL_LOG"

tail -n0 -F "$LOG_FILE" | grep --line-buffered "Launching agent" > "$TAIL_LOG" &
TAIL_PID=$!

cleanup_tail() {
  if kill -0 "$TAIL_PID" 2>/dev/null; then
    kill "$TAIL_PID" 2>/dev/null || true
    wait "$TAIL_PID" 2>/dev/null || true
  fi
}

trap cleanup_tail EXIT

echo
echo ">>> Launch ./target/dev-fast/code (or preferred binary) and run the commands above."
read -rp "Press Enter here when the run is fully complete: " _

cleanup_tail
trap - EXIT

SUMMARY_MISSING=false
if [[ ! -f "$COST_FILE" ]]; then
  echo "[!] No cost summary found at $COST_FILE. Did the run complete?" >&2
  SUMMARY_MISSING=true
fi

REPORT="$ART_DIR/report.txt"
{
  printf "SPEC: %s\n" "$SPEC"
  printf "Run:  %s\n\n" "$STAMP"
  if $SUMMARY_MISSING; then
    printf "Cost summary missing. Inspect TUI output and rerun if necessary.\n"
  else
    TOTAL=$(jq -r '.total_spent // 0' "$COST_FILE")
    BUDGET=$(jq -r '.budget // 0' "$COST_FILE")
    UTIL=$(jq -r '.utilization // 0' "$COST_FILE")
    CALLS=$(jq -r '.call_count // 0' "$COST_FILE")
    DURATION_SEC=$(jq -r '.duration // 0' "$COST_FILE")

    TOTAL_FMT=$(awk "BEGIN {printf \"%.2f\", $TOTAL}")
    BUDGET_FMT=$(awk "BEGIN {printf \"%.2f\", $BUDGET}")
    UTIL_PCT=$(awk "BEGIN {printf \"%.1f\", $UTIL * 100}")
    DURATION_MIN=$(awk "BEGIN {printf \"%.2f\", $DURATION_SEC / 60}")

    printf "Total spend: $%s (%.1f%% of $%s budget)\n" "$TOTAL_FMT" "$UTIL_PCT" "$BUDGET_FMT"
    printf "Call count:  %s\n" "$CALLS"
    printf "Duration:    %s seconds (~%s minutes)\n\n" "$DURATION_SEC" "$DURATION_MIN"

    printf "Per-stage spend:\n"
    jq -r '.per_stage | to_entries[] | @tsv' "$COST_FILE" |
      while IFS=$'\t' read -r stage value; do
        printf "  - %-12s $%.2f\n" "$stage" "${value:-0}"
      done
    printf "\nPer-model spend:\n"
    jq -r '.per_model | to_entries[] | @tsv' "$COST_FILE" |
      while IFS=$'\t' read -r model value; do
        printf "  - %-20s $%.2f\n" "$model" "${value:-0}"
      done

    if jq -e '.stage_notes | length > 0' "$COST_FILE" >/dev/null 2>&1; then
      printf "\nStage notes:\n"
      jq -r '.stage_notes[] | "  - \(.stage): effort=\(.aggregator_effort // \"n/a\"), escalation=\(.escalation_reason // \"n/a\")"' "$COST_FILE"
    fi
  fi
} > "$REPORT"

cat "$REPORT"

if $RUN_MEMORY; then
  echo "\n[4/4] Capturing local-memory snapshots."
  if ! command -v local-memory >/dev/null 2>&1; then
    echo "local-memory CLI not found; skipping snapshots." >&2
  else
    for stage in plan tasks validate; do
      OUT_FILE="$ART_DIR/local-memory-${stage}.txt"
      echo "# local-memory search \"spec:$SPEC stage:$stage\" --limit 5" > "$OUT_FILE"
      if local-memory search "spec:$SPEC stage:$stage" --limit 5 >> "$OUT_FILE" 2>&1; then
        echo "  Saved local-memory output for $stage -> $OUT_FILE"
      else
        echo "  Failed to capture local-memory for $stage (see $OUT_FILE)" >&2
      fi
    done
  fi
else
  echo "Skipping local-memory snapshots (--no-memory)."
fi

echo "\nArtifacts recorded under $ART_DIR"
echo "Done."
