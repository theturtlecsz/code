# Implementation Prompt — codex-rs (Stage0 + speckit.auto)

You are working in the **codex-rs** repo (typically at `~/code`).
Users run the **`code`** CLI.

Your goal is to align `/speckit.auto` and Stage0 with the convergence north-star:
- Stage0 Tier2 (NotebookLM) is enabled by default but must fail closed.
- System artifacts are stored as pointer memories in local-memory (domain `spec-tracker`, tag `system:true`).
- Stage0 Tier1 retrieval excludes system memories by default.

Constraints:
- local-memory is accessed via CLI + REST only (no MCP).
- Do not add a general-notebook fallback. Missing notebook mapping must skip Tier2 with explicit diagnostics.
- Do not duplicate policy logic that belongs in localmemory-policy; conform to the matrix.

Tasks:
1) Add a **`code doctor`** (or `code stage0 doctor`) helper that checks:
   - local-memory REST health
   - domain resolution
   - notebooklm service `/health/ready`
   - notebook mapping exists for current domain/spec
   - optional smoke `ask`
2) Stage0 Tier2 orchestration:
   - attempt Tier2 by default
   - if prerequisites missing, skip Tier2 and continue Tier1
   - emit structured diagnostics
3) System pointer memory:
   - write a memory to domain `spec-tracker` with tags `system:true`, `spec:<id>`, `stage:0`, `artifact:*`
   - store only pointers + short summary (no raw Divine Truth text)
4) Tests:
   - Tier1-only success when notebooklm is unavailable
   - Tier2 invoked when configured
   - system memory excluded from Tier1 retrieval

Reference:
- Use `CONVERGENCE_MATRIX.yaml` as the source of truth.
