# SPEC-940 Performance Baselines

**Date**: 2025-11-28
**Environment**: Linux 6.8.12-8-pve
**Commit**: After Phase 2 implementation

## Summary

All operations significantly exceed performance targets.

| Operation | Mean (ms) | Stddev (ms) | P95 (ms) | Max (ms) | Target (ms) | Margin |
|-----------|-----------|-------------|----------|----------|-------------|--------|
| spawn_echo_command | 0.92 | 0.06 | 0.94 | 1.07 | <50 | 54x |
| sqlite_write | 0.04 | 0.00 | 0.04 | 0.05 | <30 | 750x |
| sqlite_batch (10 ops) | 0.40 | 0.03 | 0.42 | 0.49 | <30 | 75x |
| config_parse | 0.05 | 0.00 | 0.06 | 0.06 | <10 | 200x |

## Detailed Results

### P0: DirectProcessExecutor Spawn Performance

```
Operation: spawn_echo_command
Iterations: 10 (+ 2 warmup)
Mean:   0.92ms
Stddev: 0.06ms
Min:    0.86ms
P50:    0.88ms
P95:    0.94ms
Max:    1.07ms

Target: <50ms mean, <100ms max
Result: ✅ PASS (54x better than target)
```

**Analysis**: Using tokio::process::Command with minimal `echo` command shows
sub-millisecond spawn overhead. The DirectProcessExecutor implementation
(SPEC-936) eliminates tmux overhead entirely.

### P1: SQLite Consensus Write Performance

```
Operation: sqlite_consensus_write (single insert)
Iterations: 10 (+ 2 warmup)
Mean:   0.04ms
Stddev: 0.00ms
Min:    0.04ms
P50:    0.04ms
P95:    0.04ms
Max:    0.05ms

Target: <30ms mean, <100ms max
Result: ✅ PASS (750x better than target)
```

**Analysis**: SQLite with WAL mode and connection pooling provides
excellent write performance. The SPEC-934 storage consolidation
choice is validated.

### P1: SQLite Batch Transaction Performance

```
Operation: sqlite_batch_transaction (10 inserts)
Iterations: 10 (+ 2 warmup)
Mean:   0.40ms
Stddev: 0.03ms
Min:    0.38ms
P50:    0.39ms
P95:    0.42ms
Max:    0.49ms

Target: <30ms for batch
Result: ✅ PASS (75x better than target)
```

**Analysis**: Transaction batching amortizes overhead effectively.
10 operations in 0.40ms = 0.04ms per operation, consistent with
single-write benchmark.

### P1: Config Parsing Performance

```
Operation: config_parse (TOML file read + parse)
Iterations: 10 (+ 2 warmup)
Mean:   0.05ms
Stddev: 0.00ms
Min:    0.05ms
P50:    0.05ms
P95:    0.06ms
Max:    0.06ms

Target: <10ms mean, <50ms max
Result: ✅ PASS (200x better than target)
```

**Analysis**: TOML parsing is negligible. File I/O dominates but
remains sub-millisecond for small config files.

## Validation Commands

```bash
# Run all benchmarks
cd codex-rs
cargo test -p codex-core --test spec940_benchmarks -- --nocapture

# Run specific benchmark
cargo test -p codex-core --test spec940_benchmarks benchmark_spawn -- --nocapture
```

## Regression Detection

Any PR that causes:
- Spawn mean > 5ms: **FAIL** (5x current baseline)
- SQLite write mean > 1ms: **FAIL** (25x current baseline)
- Config parse mean > 1ms: **FAIL** (20x current baseline)

These thresholds provide margin while catching actual regressions.

## Next Steps

1. ✅ Phase 1: BenchmarkHarness implementation
2. ✅ Phase 2: Baseline measurements
3. ⏳ Phase 3: CI integration with regression detection
