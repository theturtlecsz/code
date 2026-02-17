# ADR 012: Emergency Protocol (Class E)

## Context
Strict adherence to "Milestone Boundaries" for Class 2 changes prevents rapid response to Critical Vulnerabilities (e.g., `log4j`, `openssl` bugs) or Production Outages if a milestone is mid-flight. The system must not be paralyzed by its own process during a crisis.

## Decision
We define **Class E (Emergency)** changes.

1.  **Trigger:**
    *   **CVSS Score > 7.0** (High/Critical) detected in dependencies.
    *   **Production Outage** confirmed by external monitor/user override.
2.  **Privilege:** Class E changes BYPASS the "Milestone Boundary" rule. They can be injected *immediately* into the Primary Thread.
3.  **Constraint:**
    *   Must be **Atomic** (minimal file set).
    *   Must include a **Rollback Script**.
    *   Must trigger an **Immediate Notification** (regardless of user settings).

## Consequences
*   **Positive:** System remains viable in real-world ops conditions.
*   **Negative:** Risk of breaking the current milestone's stability.
*   **Mitigation:** The system snapshots the current state before applying Class E, allowing a clean resume after the hotfix.
