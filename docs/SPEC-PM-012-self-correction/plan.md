# Plan: SPEC-PM-012 Self-Correction

## Approach Options

1. **Escalate immediately on first failure**  
   Rejected: low autonomy.
2. **Bounded retry with failure-context feedback (chosen)**  
   Balanced autonomy and control.
3. **Unbounded autonomous repair**  
   Rejected: unsafe and hard to reason about.

## Chosen Path

Implement bounded retry orchestration with signature-aware loop guards and structured escalation output after exhaustion.

## Milestone Boundary Semantics

- Self-correction operates within current milestone scope only.
- Detected Class 2 fix proposals are queued for boundary decision, not auto-adopted.
- Emergency hotfixes can route via Class E if criteria are met.

## Rollout / Migration

- Add retry context capture and signature hashing first.
- Enable retry loop for build/test stages.
- Enable escalation packet and notification policy last.

## Validation Mapping

| Requirement | Validation | Artifact |
| --- | --- | --- |
| AC1 | `cd codex-rs && cargo test -p codex-core retry::bounded_loop` | `docs/SPEC-PM-012-self-correction/artifacts/tests/bounded-loop.txt` |
| AC2 | `cd codex-rs && cargo test -p codex-core retry::context_feedback` | `docs/SPEC-PM-012-self-correction/artifacts/tests/context-feedback.txt` |
| AC3 | `cd codex-rs && cargo test -p codex-core retry::early_success` | `docs/SPEC-PM-012-self-correction/artifacts/tests/early-success.txt` |
| AC4 | `cd codex-rs && cargo test -p codex-core retry::escalation_packet` | `docs/SPEC-PM-012-self-correction/artifacts/escalations/<run_id>.md` |
| AC5 | `cd codex-rs && cargo test -p codex-core retry::signature_guard` | `docs/SPEC-PM-012-self-correction/artifacts/tests/signature-guard.txt` |
