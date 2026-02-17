# ADR 0009: Milestones, trajectories, and Class 2 adoption boundaries

## Context
We want discovery and major architectural moves without thrash.

## Decision
Milestones are typed (Ship/Decision/Artifact). Milestone boundary is when milestone is Done.
Class 2 changes may only be adopted at milestone boundaries and follow Decision→Migration→Ship patterns.

## Consequences
- Prevents constant architecture churn mid-milestone.
- Keeps autonomy convergent.
