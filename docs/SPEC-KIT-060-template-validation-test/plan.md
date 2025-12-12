# Plan: Template-Based Spec Generation Validation and Adoption

**SPEC-ID**: SPEC-KIT-060-template-validation-test  
**Plan Version**: 1.0  
**Created**: 2025-10-27

---

## Inputs

- Spec: `docs/SPEC-KIT-060-template-validation-test/spec.md`
- Comparison: `docs/SPEC-KIT-060-template-validation-test/final-comparison.md`
- Baseline: `docs/SPEC-KIT-060-template-validation-test/baseline-results.md`
- Template: `templates/plan-template.md`
- Spec‑Kit modules (read‑only reference):
  - `codex-rs/tui/src/chatwidget/spec_kit/handler.rs`
  - `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs`
  - `codex-rs/tui/src/chatwidget/spec_kit/state.rs`
  - `codex-rs/tui/src/chatwidget/spec_kit/commands/quality.rs`
- Policies: `MEMORY-POLICY.md`, `docs/spec-kit/evidence-policy.md`, `docs/spec-kit/testing-policy.md`, `docs/architecture/async-sync-boundaries.md`
- Metric used as canonical: template generation is 50% faster (15 min vs 30 min baseline) per `final-comparison.md`.
- Note: There is no PRD.md for this SPEC; plan derives from `spec.md` + evidence.

---

## Work Breakdown

### Step 1: Confirm Phase 1 Results (Validation) — COMPLETE
- Reconcile speedup discrepancy (50% vs 55%) by re-running one quick A/B on a small SPEC and updating a single source of truth (this plan assumes 50% as canonical).
- Record the reconciliation note in `final-comparison.md` and cross‑reference from `spec.md`.

### Step 2: Port /speckit.clarify to templates
- Implement template-driven output using `templates/clarify-template.md`.
- Map existing prompt assembly in `handler.rs` to template sections; ensure all placeholders are populated.
- Success signal: Clarify artifact matches template sections; no raw placeholders remain.
- Est. effort: 1–2 days.

### Step 3: Port /speckit.analyze to templates
- Use `templates/analyze-template.md` to structure analysis.
- Validate PRD↔Spec, Plan↔Spec, Tasks↔Acceptance sections align with template.
- Success signal: Analyze artifact matches template; cross‑artifact checks rendered.
- Est. effort: 1–2 days.

### Step 4: Port /speckit.checklist to templates
- Use `templates/checklist-template.md` for requirement quality and scores.
- Success signal: Checklist artifact matches template; scoring present and consistent.
- Est. effort: 1 day.

### Step 5: Ensure /speckit.auto uses templates for plan/tasks
- Verify `plan` uses `templates/plan-template.md`; `tasks` uses `templates/tasks-template.md`.
- Success signal: Auto pipeline writes correctly structured plan/tasks artifacts for any SPEC.
- Est. effort: 1 day.

### Step 6: Add validation and telemetry
- Add a post‑generation scan that fails if any `[PLACEHOLDER]` remains.
- Emit per‑stage telemetry and persist consensus verdicts per existing evidence schema.
- Success signal: No placeholder leakage; verdict + telemetry files present under evidence.

### Step 7: Rollout controls
- Gate with config flag (e.g., `spec_kit.use_templates = true`). Default ON post‑validation.
- Document rollback steps in `spec.md` (already present) and release notes.

---

## Technical Design

### Data Model Changes
- None.

### API Contracts
- No external API changes. TUI command contracts unchanged; only artifact structure is enforced via templates.

### Component Architecture
- Command entry: `codex-rs/tui/src/chatwidget/spec_kit/commands/quality.rs` (clarify/analyze/checklist) delegates to `handler.rs`.
- Integration point: `handler.rs` assembles prompts and writes artifacts; extend to load the appropriate template file and fill sections.
- Consensus + persistence: unchanged; `consensus.rs` continues to synthesize and `evidence.rs` persists.
- Config: introduce `spec_kit.use_templates` flag read by `handler.rs` during command execution.

---

## Acceptance Mapping

- Update all existing commands to use templates → Steps 2–4 complete; artifacts match templates; no placeholder leakage.
- Proceed to Phase 2 (from `spec.md`) → Steps 2–6 complete; flag set ON; evidence and telemetry emitted.
- Time ≤ baseline time → Already validated by Phase 1; spot‑check on real SPEC during Step 2 to confirm no regression.
- Maintain evidence discipline → Evidence artifacts and verdicts stored; footprint monitored under 25 MB per SPEC.

---

## Risks & Unknowns

### Risk 1: Template drift
- Impact: Medium; Mitigation: CI check that verifies all template placeholders are recognized and populated.

### Risk 2: Placeholder leakage
- Impact: High; Mitigation: Post‑render placeholder scan and fail‑safe; add checklist item in `/speckit.validate`.

### Risk 3: Consensus regressions
- Impact: High; Mitigation: Run existing consensus tests against templated outputs; compare agreements/conflicts rate.

### Risk 4: Evidence growth
- Impact: Low; Mitigation: Follow evidence policy; prune intermediate artifacts if nearing 25 MB.

### Unknown: Exact effort per command
- Resolution: Timebox first port (/clarify), calibrate subsequent estimates.

---

## Multi-Agent Consensus

### Agreements
- Adopt templates due to 50% speed improvement with no quality loss.
- Port `/speckit.clarify`, `/speckit.analyze`, `/speckit.checklist`; ensure `/speckit.auto` uses `plan/tasks` templates.
- Keep local‑memory as the sole curated knowledge system; persist consensus artifacts with importance ≥ 8.

### Conflicts Resolved
- Speedup discrepancy (50% vs 55%): Use 50% from `final-comparison.md` as canonical; schedule a quick confirmatory run before closing SPEC.
- Placeholder rigor: Strictly fail on any leftover placeholders; suggest content but do not silently auto‑fill in Phase 2.

---

## Exit Criteria

- All three commands produce template‑conformant artifacts with zero placeholders.
- `/speckit.auto` generates template‑conformant plan/tasks artifacts.
- Consensus verdicts and telemetry stored; no evidence policy violations.
- One confirmatory timing run shows no regression vs validated 50% improvement.
- Plan approved; ready to proceed to `/speckit.tasks` for SPEC‑KIT‑060.

---

## Evidence References

- `docs/SPEC-KIT-060-template-validation-test/spec.md`
- `docs/SPEC-KIT-060-template-validation-test/final-comparison.md`
- `docs/SPEC-KIT-060-template-validation-test/baseline-results.md`
- `templates/plan-template.md`
- `codex-rs/tui/src/chatwidget/spec_kit/handler.rs`
- `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs`
- `MEMORY-POLICY.md`, `docs/spec-kit/evidence-policy.md`

