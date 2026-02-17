# ADR 0006: Sacred anchors and epochs

## Context
Primary trust-killer is context loss and drift.

## Decision
User intent summary and success criteria are sacred. Material changes create a new epoch; the system must not silently drift.

## Consequences
- Prevents “it forgot what we were doing.”
- Enables meaningful progress/learning measurement per epoch.
