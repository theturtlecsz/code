#!/usr/bin/env bash
# DEPRECATED (SPEC-KIT-902): This script is replaced by native Rust implementation
# in codex-rs/tui/src/chatwidget/spec_kit/evidence.rs (check_spec_evidence_limit)
# Kept for manual CLI use and backward compatibility.
# Use /spec-evidence-stats in TUI for native implementation.

set -euo pipefail

EVIDENCE_DIR="docs/SPEC-OPS-004-integrated-coder-hooks/evidence"

usage() {
  cat <<'USAGE' >&2
Usage: evidence_stats.sh [--spec <SPEC-ID>]

Summarise guardrail and consensus evidence sizes. With --spec, only inspect
artifacts for the given SPEC (case-sensitive).
USAGE
  exit 1
}

SPEC_FILTER=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --spec)
      [[ $# -ge 2 ]] || usage
      SPEC_FILTER="$2"
      shift 2
      ;;
    --help|-h)
      usage
      ;;
    *)
      usage
      ;;
  esac
done

if [[ ! -d "${EVIDENCE_DIR}" ]]; then
  echo "Evidence directory not found: ${EVIDENCE_DIR}" >&2
  exit 1
fi

print_section() {
  local title="$1"
  shift
  echo "=== ${title} ==="
  "$@"
  echo
}

list_targets() {
  local base="$1"; shift
  if [[ -n "${SPEC_FILTER}" ]]; then
    find "${base}" -maxdepth 1 -mindepth 1 -type d -name "${SPEC_FILTER}" -print
  else
    find "${base}" -maxdepth 1 -mindepth 1 -type d -print | sort
  fi
}

summarise_dir() {
  local dir="$1"; shift
  if [[ -d "${dir}" ]]; then
    du -sh "${dir}" | awk '{print $2 "\t" $1}'
  fi
}

print_section "Evidence root size" du -sh "${EVIDENCE_DIR}"

command_base="${EVIDENCE_DIR}/commands"
if [[ -d "${command_base}" ]]; then
  output=""
  while IFS= read -r spec; do
    [[ -z "${spec}" ]] && continue
    size=$(du -sh "${spec}" | awk '{print $1}')
    output+="$(basename "${spec}")\t${size}\n"
  done <<<"$(list_targets "${command_base}")"
  if [[ -n "${output}" ]]; then
    print_section "Command telemetry size by SPEC" printf '%b' "${output}"
  fi
fi

consensus_base="${EVIDENCE_DIR}/consensus"
if [[ -d "${consensus_base}" ]]; then
  size_output=""
  count_output="SPEC\tFILES\n"
  while IFS= read -r spec; do
    [[ -z "${spec}" ]] && continue
    base_name=$(basename "${spec}")
    size=$(du -sh "${spec}" | awk '{print $1}')
    size_output+="${base_name}\t${size}\n"
    count=$(find "${spec}" -type f -name "*.json" | wc -l | tr -d ' ')
    count_output+="${base_name}\t${count}\n"
  done <<<"$(list_targets "${consensus_base}")"

  if [[ -n "${size_output}" ]]; then
    print_section "Consensus size by SPEC" printf '%b' "${size_output}"
  fi
  print_section "Consensus artifact counts" printf '%b' "${count_output}"

  warn_threshold_mb=40
  fail_threshold_mb=50
  echo "=== Policy Compliance (40 MB warn / 50 MB limit) - SPEC-KIT-909 ==="
  warn_count=0
  fail_count=0
  while IFS= read -r spec; do
    [[ -z "${spec}" ]] && continue
    base_name=$(basename "${spec}")

    # Calculate total size (consensus + commands)
    consensus_size=$(du -sb "${consensus_base}/${base_name}" 2>/dev/null | awk '{print $1}' || echo "0")
    commands_size=$(du -sb "${command_base}/${base_name}" 2>/dev/null | awk '{print $1}' || echo "0")
    total_bytes=$((consensus_size + commands_size))
    total_mb=$(awk "BEGIN {printf \"%.1f\", ${total_bytes} / 1048576}")

    exceeds_fail=$(awk "BEGIN {print (${total_mb} > ${fail_threshold_mb}) ? 1 : 0}")
    exceeds_warn=$(awk "BEGIN {print (${total_mb} > ${warn_threshold_mb}) ? 1 : 0}")

    if [[ ${exceeds_fail} -eq 1 ]]; then
      echo "❌ ${base_name}: ${total_mb} MB (exceeds ${fail_threshold_mb} MB limit)"
      echo "    Action: Archive immediately (scripts/spec_ops_004/evidence_archive.sh)"
      echo "    Policy: docs/spec-kit/evidence-policy.md"
      fail_count=$((fail_count + 1))
    elif [[ ${exceeds_warn} -eq 1 ]]; then
      echo "⚠️  ${base_name}: ${total_mb} MB (above ${warn_threshold_mb} MB warning)"
      echo "    Action: Plan cleanup before exceeding ${fail_threshold_mb} MB"
      warn_count=$((warn_count + 1))
    fi
  done <<<"$(list_targets "${consensus_base}")"

  if [[ ${fail_count} -eq 0 && ${warn_count} -eq 0 ]]; then
    echo "✅ All SPECs within guardrails"
  elif [[ ${fail_count} -eq 0 ]]; then
    echo "⚠️  ${warn_count} SPEC(s) above ${warn_threshold_mb} MB warning"
  else
    echo "❌ ${fail_count} SPEC(s) above ${fail_threshold_mb} MB limit"
  fi
  echo
fi
