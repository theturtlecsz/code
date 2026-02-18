//! PM-006: Golden fixture tests for packet persistence.

use codex_spec_kit::packet::{Packet, PacketIoError, anchor_guard, io, schema};
use tempfile::TempDir;

#[test]
fn write_read_roundtrip() {
    let dir = TempDir::new().expect("temp dir");
    let speckit_dir = dir.path().join(".speckit");

    let mut packet = Packet::new(
        "roundtrip-001".into(),
        "Test roundtrip persistence".into(),
        vec!["Data survives write/read cycle".into()],
    );
    packet.add_milestone("milestone-1".into(), true);

    io::write_packet(&speckit_dir, &mut packet).expect("write should succeed");

    let loaded = io::read_packet(&speckit_dir).expect("read should succeed");
    assert_eq!(loaded.header.packet_id, "roundtrip-001");
    assert_eq!(loaded.header.epoch, 2); // incremented from 1 on write
    assert_eq!(
        loaded.sacred_anchors.intent_summary,
        "Test roundtrip persistence"
    );
    assert_eq!(loaded.milestones.len(), 1);
    assert_eq!(loaded.milestones[0].name, "milestone-1");
    assert!(loaded.milestones[0].required_for_class2);
}

#[test]
fn read_missing_packet_returns_not_found() {
    let dir = TempDir::new().expect("temp dir");
    let speckit_dir = dir.path().join(".speckit");

    let err = io::read_packet(&speckit_dir).unwrap_err();
    assert!(matches!(err, PacketIoError::NotFound { .. }));
}

#[test]
fn read_corrupted_packet_returns_error() {
    let dir = TempDir::new().expect("temp dir");
    let speckit_dir = dir.path().join(".speckit");
    std::fs::create_dir_all(&speckit_dir).expect("create dir");
    std::fs::write(speckit_dir.join("packet.json"), "not valid json {{{")
        .expect("write corrupt file");

    let err = io::read_packet(&speckit_dir).unwrap_err();
    assert!(matches!(err, PacketIoError::Corrupted { .. }));
}

#[test]
fn anchor_guard_blocks_direct_modification() {
    let original = Packet::new(
        "guard-001".into(),
        "Original intent".into(),
        vec!["Original criteria".into()],
    );
    let mut modified = original.clone();
    modified.sacred_anchors.intent_summary = "Changed intent".into();

    let err = anchor_guard::check_anchor_integrity(&original, &modified).unwrap_err();
    assert!(err.to_string().contains("intent_summary"));
}

#[test]
fn anchor_guard_allows_amendment() {
    let original = Packet::new(
        "guard-002".into(),
        "Original intent".into(),
        vec!["Original criteria".into()],
    );
    let mut amended = original.clone();
    anchor_guard::amend_intent(
        &mut amended,
        "Amended intent".into(),
        "Requirements changed".into(),
    );

    assert!(anchor_guard::check_anchor_integrity(&original, &amended).is_ok());
    assert_eq!(amended.sacred_anchors.intent_summary, "Amended intent");
    assert_eq!(amended.sacred_anchors.amend_history.len(), 1);
}

#[test]
fn packet_schema_version_is_set() {
    let packet = Packet::new("ver-001".into(), "test".into(), vec![]);
    assert_eq!(packet.header.schema_version, schema::SCHEMA_VERSION);
}

#[test]
fn epoch_increments_on_each_write() {
    let dir = TempDir::new().expect("temp dir");
    let speckit_dir = dir.path().join(".speckit");

    let mut packet = Packet::new("epoch-001".into(), "test".into(), vec!["c1".into()]);
    assert_eq!(packet.header.epoch, 1);

    io::write_packet(&speckit_dir, &mut packet).expect("first write");
    assert_eq!(packet.header.epoch, 2);

    io::write_packet(&speckit_dir, &mut packet).expect("second write");
    assert_eq!(packet.header.epoch, 3);

    let loaded = io::read_packet(&speckit_dir).expect("read");
    assert_eq!(loaded.header.epoch, 3);
}
