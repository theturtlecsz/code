# Evidence Cleanup Notes

**Date**: 2025-10-29
**Status**: All SPECs within 25MB soft limit ✅

---

## Current State

Per `/spec-evidence-stats`, all SPEC directories are currently within the 25 MB soft limit. No immediate cleanup required.

---

## Evidence Retention Policy

Per [docs/spec-kit/evidence-policy.md](../../spec-kit/evidence-policy.md):

### Retention Schedule
- **Active**: Keep all evidence for active/in-progress SPECs
- **Recent**: Keep last 2 baselines per stage for completed SPECs
- **Aged**: Archive baselines >30 days old
- **Archived**: Compress and offload >90 days old
- **Purged**: Remove >180 days old (or when total >25MB per SPEC)

### Cleanup Process

When a SPEC exceeds 25MB soft limit:

1. **Identify Duplicates**:
   ```bash
   find docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-###/ \
     -name "baseline_*.md" | sort
   ```

2. **Keep Recent** (last 2 baselines per stage):
   ```bash
   cd docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-###/
   ls -t baseline_*.md | tail -n +3 > old_baselines.txt
   ```

3. **Archive Old Baselines**:
   ```bash
   mkdir -p ../../archive/SPEC-KIT-###/
   cat old_baselines.txt | xargs -I {} mv {} ../../archive/SPEC-KIT-###/
   ```

4. **Compress Archive**:
   ```bash
   cd ../../archive/
   tar -czf SPEC-KIT-###-baselines-$(date +%Y%m%d).tar.gz SPEC-KIT-###/
   rm -rf SPEC-KIT-###/
   ```

---

## Evidence Statistics

Run `/spec-evidence-stats` to check current footprint:

```bash
/spec-evidence-stats                    # All SPECs
/spec-evidence-stats --spec SPEC-KIT-###  # Specific SPEC
```

---

## Large Evidence Directories

From doc-curator scan, the following SPEC directories have many duplicate baseline files:

### SPEC-KIT-045-mini
- Contains 18+ baseline files from iterative testing
- Many are similar (testing framework iterations)
- **Recommendation**: Keep last 2, archive rest when approaching 25MB

### SPEC-KIT-025, SPEC-KIT-030, SPEC-KIT-067
- Multiple baseline files from development iterations
- **Recommendation**: Apply retention policy if growth continues

---

## Monitoring

**Weekly Check**:
```bash
/spec-evidence-stats | grep "WARNING"
```

**Monthly Review**:
1. Run `/spec-evidence-stats --all`
2. Identify SPECs >20MB (approaching limit)
3. Apply retention policy to aging evidence
4. Document cleanup in this file

---

## Cleanup History

### 2025-10-29
- Initial assessment: All SPECs <25MB ✅
- No cleanup required
- Policy documented for future reference

---

## See Also

- [Evidence Policy](../../spec-kit/evidence-policy.md) - Full retention policy
- [/spec-evidence-stats](../../../scripts/spec_ops_004/evidence_stats.sh) - Monitoring script
- [CLAUDE.md](../../../CLAUDE.md) - Evidence expectations
