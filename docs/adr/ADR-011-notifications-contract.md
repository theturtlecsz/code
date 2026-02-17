# ADR 011: Notifications and daily recap contract

## Context
Non-blocking guidance requires a notification policy.

## Decision
Immediate notifications only for:
- major decision milestone ready AND posture threshold met
- critical security issues (secrets, RCE patterns)
Everything else is non-interrupting; provide a daily recap.

## Consequences
- Notifications remain rare and high-signal.
- User stays informed without interruption overload.
