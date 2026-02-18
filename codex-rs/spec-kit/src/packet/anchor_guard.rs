//! PM-006 T003: Sacred anchor immutability enforcement.
//!
//! Sacred anchors (intent_summary, success_criteria) are immutable
//! without an explicit amendment flag. This module provides the guard
//! that blocks modifications and the amendment workflow.

use super::schema::{AmendmentRecord, Packet};

/// Errors from anchor guard enforcement.
#[derive(Debug, thiserror::Error)]
pub enum AnchorGuardError {
    /// Attempted to modify a sacred field without the amendment flag.
    #[error(
        "Sacred anchor '{field}' cannot be modified without explicit amendment. \
         Use amend_intent() or amend_success_criteria() with a reason."
    )]
    ModificationBlocked { field: String },
}

/// Check whether the sacred anchors of `updated` differ from `original`.
///
/// Returns `Err` if any sacred field was modified without going through
/// the amendment workflow (i.e., without a corresponding new entry in
/// `amend_history`).
pub fn check_anchor_integrity(original: &Packet, updated: &Packet) -> Result<(), AnchorGuardError> {
    // Check intent_summary
    if original.sacred_anchors.intent_summary != updated.sacred_anchors.intent_summary {
        // Was there a new amendment record for this field?
        let has_amendment = updated.sacred_anchors.amend_history.len()
            > original.sacred_anchors.amend_history.len()
            && updated
                .sacred_anchors
                .amend_history
                .last()
                .is_some_and(|r| r.field == "intent_summary");

        if !has_amendment {
            return Err(AnchorGuardError::ModificationBlocked {
                field: "intent_summary".into(),
            });
        }
    }

    // Check success_criteria
    if original.sacred_anchors.success_criteria != updated.sacred_anchors.success_criteria {
        let has_amendment = updated.sacred_anchors.amend_history.len()
            > original.sacred_anchors.amend_history.len()
            && updated
                .sacred_anchors
                .amend_history
                .last()
                .is_some_and(|r| r.field == "success_criteria");

        if !has_amendment {
            return Err(AnchorGuardError::ModificationBlocked {
                field: "success_criteria".into(),
            });
        }
    }

    Ok(())
}

/// Amend the intent summary with a recorded reason.
///
/// This is the ONLY way to modify `intent_summary` after initial creation.
pub fn amend_intent(packet: &mut Packet, new_intent: String, reason: String) {
    let previous = packet.sacred_anchors.intent_summary.clone();
    packet.sacred_anchors.amend_history.push(AmendmentRecord {
        amended_at: chrono::Utc::now().to_rfc3339(),
        reason,
        field: "intent_summary".into(),
        previous_value: previous,
    });
    packet.sacred_anchors.intent_summary = new_intent;
}

/// Amend the success criteria with a recorded reason.
///
/// This is the ONLY way to modify `success_criteria` after initial creation.
pub fn amend_success_criteria(packet: &mut Packet, new_criteria: Vec<String>, reason: String) {
    let previous =
        serde_json::to_string(&packet.sacred_anchors.success_criteria).unwrap_or_default();
    packet.sacred_anchors.amend_history.push(AmendmentRecord {
        amended_at: chrono::Utc::now().to_rfc3339(),
        reason,
        field: "success_criteria".into(),
        previous_value: previous,
    });
    packet.sacred_anchors.success_criteria = new_criteria;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_packet() -> Packet {
        Packet::new(
            "test-001".into(),
            "Build feature X".into(),
            vec!["Tests pass".into(), "Docs updated".into()],
        )
    }

    #[test]
    fn unmodified_packet_passes_guard() {
        let original = sample_packet();
        let updated = original.clone();
        assert!(check_anchor_integrity(&original, &updated).is_ok());
    }

    #[test]
    fn direct_intent_modification_blocked() {
        let original = sample_packet();
        let mut updated = original.clone();
        updated.sacred_anchors.intent_summary = "Something else".into();

        let err = check_anchor_integrity(&original, &updated).unwrap_err();
        assert!(err.to_string().contains("intent_summary"));
    }

    #[test]
    fn direct_criteria_modification_blocked() {
        let original = sample_packet();
        let mut updated = original.clone();
        updated.sacred_anchors.success_criteria = vec!["Different criteria".into()];

        let err = check_anchor_integrity(&original, &updated).unwrap_err();
        assert!(err.to_string().contains("success_criteria"));
    }

    #[test]
    fn amend_intent_passes_guard() {
        let original = sample_packet();
        let mut updated = original.clone();
        amend_intent(
            &mut updated,
            "Build feature Y instead".into(),
            "Requirements changed".into(),
        );

        assert!(check_anchor_integrity(&original, &updated).is_ok());
        assert_eq!(updated.sacred_anchors.amend_history.len(), 1);
        assert_eq!(
            updated.sacred_anchors.amend_history[0].field,
            "intent_summary"
        );
    }

    #[test]
    fn amend_criteria_passes_guard() {
        let original = sample_packet();
        let mut updated = original.clone();
        amend_success_criteria(
            &mut updated,
            vec!["New criteria".into()],
            "Scope refined".into(),
        );

        assert!(check_anchor_integrity(&original, &updated).is_ok());
        assert_eq!(updated.sacred_anchors.amend_history.len(), 1);
        assert_eq!(
            updated.sacred_anchors.amend_history[0].field,
            "success_criteria"
        );
    }
}
