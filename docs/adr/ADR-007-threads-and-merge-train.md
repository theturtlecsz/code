# ADR 0007: Thread model and merge privileges

## Context
We want autonomy without shipping surprises.

## Decision
One primary development thread per project (merge train). Many research/review threads. Research/review never merge.

Auto-merge only in primary thread when attended, and only after recap.

## Consequences
- Safe unattended behavior.
- Clear separation of exploration vs shipping.
