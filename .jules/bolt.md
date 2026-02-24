## 2024-05-23 - Micro-optimizing BinaryHeap allocation
**Learning:** Replacing `pop()` + `push()` with `peek_mut()` and String reuse in a hot loop (file search) didn't show clear E2E latency improvement. The cost of fuzzy matching likely dwarfs the allocation overhead for small heaps (100 items).
**Action:** Use micro-benchmarks (criterion) to measure such changes, as E2E noise masks the improvement.
