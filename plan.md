# Architecture Review Board - Pass 2 Plan

## Objective
Facilitate interactive decision-making on 12+ architectural questions for Codex-RS/Spec-Kit using cached Pass 1 research, producing:
1. Decisions table with chosen options and rationale
2. Resurrection Map porting archived capabilities to Memvid-first equivalents
3. Proposed updates to DECISION_REGISTER and SPEC.md

## Session 1 Scope (Current)
Questions: A1, A2, B1 (3 of ~15 total)

## Document Structure
Create ARCHITECT_REVIEW_BOARD_OUTPUT.md with:
- STATE tracker
- Decisions table (accumulating)
- Per-question output:
  - Context (why it matters for Memvid-first)
  - Options (4 minimum): description, pros, cons, best-when, risk-flags, Memvid-fit notes
  - Evidence anchors from Pass 1
  - Decision rubric (6-10 bullets)
  - User choice capture

## Neutrality Protocol
- Present facts from research separately from guidance
- Never label an option as "correct"
- Use "If you prioritize X, option Y tends to fit" framing
- Allow Option 0 (Defer) for all questions

## Questions Sequence
| Session | Questions | Status |
|---------|-----------|--------|
| 1 | A1, A2, B1 | ACTIVE |
| 2 | B2, C1, C2 | PENDING |
| 3 | D1, D2, E1 | PENDING |
| 4 | E2, F1, F2 | PENDING |
| 5 | G1, G2, G3 | PENDING |
| 6 | Synthesis + Resurrection Map | PENDING |

## Key Files Referenced
- ARCHITECT_REVIEW_RESEARCH.md (Pass 1 research)
- codex-rs/SPEC.md (Invariants, current state)
- codex-rs/docs/DECISION_REGISTER.md (D1-D112 locked)
- codex-rs/model_policy.toml (Current configuration)
