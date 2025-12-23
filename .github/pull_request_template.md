## Summary

<!-- Brief description of what this PR does -->

## Convergence Checklist

<!-- Required for all PRs - complete before merging -->

### Matrix Impact

Which convergence matrix row(s) does this PR affect?

- [ ] Stage0 (Tier1 DCC)
- [ ] Stage0 (Tier2 NotebookLM)
- [ ] System pointer memories
- [ ] local-memory integration
- [ ] Golden path (`/speckit.auto`)
- [ ] None of the above

### Interface Changes

Did you change any interface or default behavior?

- [ ] **No** - No interface changes
- [ ] **Yes** - Updated the following:
  - [ ] Bumped version field in `COMPATIBILITY.yaml` or equivalent
  - [ ] Updated canonical docs in `localmemory-policy` (or opened coordinated PR)
  - [ ] Updated `docs/convergence/README.md` pointers if needed

### Fail-Closed Semantics

If this PR touches Tier2 (NotebookLM) integration:

- [ ] Tier2 skips gracefully when service unavailable (not error)
- [ ] Tier2 skips gracefully when notebook mapping missing
- [ ] No "general notebook" or implicit fallback behavior introduced
- [ ] Diagnostics are emitted when Tier2 is skipped

### Testing

What tests prove this behavior?

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated (if applicable)
- [ ] `./scripts/convergence_check.sh` passes
- [ ] `code stage0 doctor` passes (if applicable)

## Test Plan

<!-- How did you verify this works? -->

1.
2.
3.

## Related Issues

<!-- Link any related issues: Fixes #123, Relates to #456 -->

---

<details>
<summary>Convergence Quick Reference</summary>

### Key Principles

1. **Fail Closed**: Tier2 SKIPS (not errors) when unavailable
2. **No Silent Fallback**: Never use a "general" notebook without explicit config
3. **System Pointers**: Use `domain: spec-tracker`, `system:true` tag
4. **Best Effort**: Pointer write failure must not fail Stage0

### Verification Commands

```bash
# Run convergence checks
./scripts/convergence_check.sh

# Verify Stage0 health
code stage0 doctor

# Run convergence tests
cargo test -p codex-stage0 --test convergence_acceptance
```

</details>
