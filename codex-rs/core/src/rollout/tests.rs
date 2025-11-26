#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs::File;
use std::fs::{self};
use std::io::Write;
use std::path::Path;

use tempfile::TempDir;
use time::OffsetDateTime;
use time::PrimitiveDateTime;
use time::format_description::FormatItem;
use time::macros::format_description;
use uuid::Uuid;

// ConversationItem and ConversationsPage are referenced in test assertions below
#[allow(unused_imports)]
use crate::rollout::list::ConversationItem;
#[allow(unused_imports)]
use crate::rollout::list::ConversationsPage;
use crate::rollout::list::Cursor;
use crate::rollout::list::get_conversation;
use crate::rollout::list::get_conversations;

fn write_session_file(
    root: &Path,
    ts_str: &str,
    uuid: Uuid,
    num_records: usize,
) -> std::io::Result<(OffsetDateTime, Uuid)> {
    let format: &[FormatItem] =
        format_description!("[year]-[month]-[day]T[hour]-[minute]-[second]");
    let dt = PrimitiveDateTime::parse(ts_str, format)
        .unwrap()
        .assume_utc();
    let dir = root
        .join("sessions")
        .join(format!("{:04}", dt.year()))
        .join(format!("{:02}", u8::from(dt.month())))
        .join(format!("{:02}", dt.day()));
    fs::create_dir_all(&dir)?;

    let filename = format!("rollout-{ts_str}-{uuid}.jsonl");
    let file_path = dir.join(filename);
    let mut file = File::create(file_path)?;

    // Write a valid SessionMeta RolloutLine (required for saw_session_meta)
    let session_meta = serde_json::json!({
        "timestamp": ts_str,
        "type": "session_meta",
        "payload": {
            "id": uuid.to_string(),
            "timestamp": ts_str,
            "cwd": "/tmp",
            "originator": "test",
            "cli_version": "1.0"
        }
    });
    writeln!(file, "{session_meta}")?;

    // Write an Event with AgentMessage (required for saw_user_event)
    let event = serde_json::json!({
        "timestamp": ts_str,
        "type": "event",
        "payload": {
            "id": format!("event-{}", uuid),
            "event_seq": 0,
            "msg": {
                "type": "agent_message",
                "message": "test message"
            }
        }
    });
    writeln!(file, "{event}")?;

    for i in 0..num_records {
        let rec = serde_json::json!({
            "timestamp": ts_str,
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": format!("response {i}")}]
            }
        });
        writeln!(file, "{rec}")?;
    }
    Ok((dt, uuid))
}

#[tokio::test]
async fn test_list_conversations_latest_first() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();

    // Fixed UUIDs for deterministic expectations
    let u1 = Uuid::from_u128(1);
    let u2 = Uuid::from_u128(2);
    let u3 = Uuid::from_u128(3);

    // Create three sessions across three days
    write_session_file(home, "2025-01-01T12-00-00", u1, 3).unwrap();
    write_session_file(home, "2025-01-02T12-00-00", u2, 3).unwrap();
    write_session_file(home, "2025-01-03T12-00-00", u3, 3).unwrap();

    let page = get_conversations(home, 10, None).await.unwrap();

    // Build expected paths (newest first)
    let p1 = home
        .join("sessions")
        .join("2025")
        .join("01")
        .join("03")
        .join(format!("rollout-2025-01-03T12-00-00-{u3}.jsonl"));
    let p2 = home
        .join("sessions")
        .join("2025")
        .join("01")
        .join("02")
        .join(format!("rollout-2025-01-02T12-00-00-{u2}.jsonl"));
    let p3 = home
        .join("sessions")
        .join("2025")
        .join("01")
        .join("01")
        .join(format!("rollout-2025-01-01T12-00-00-{u1}.jsonl"));

    // Verify paths are returned in correct order (newest first)
    assert_eq!(page.items.len(), 3);
    assert_eq!(page.items[0].path, p1);
    assert_eq!(page.items[1].path, p2);
    assert_eq!(page.items[2].path, p3);

    // Verify head contains the session ID
    assert!(
        page.items[0].head[0]["id"]
            .as_str()
            .unwrap()
            .contains(&u3.to_string())
    );
    assert!(
        page.items[1].head[0]["id"]
            .as_str()
            .unwrap()
            .contains(&u2.to_string())
    );
    assert!(
        page.items[2].head[0]["id"]
            .as_str()
            .unwrap()
            .contains(&u1.to_string())
    );

    // Verify cursor and counts
    let expected_cursor: Cursor =
        serde_json::from_str(&format!("\"2025-01-01T12-00-00|{u1}\"")).unwrap();
    assert_eq!(page.next_cursor, Some(expected_cursor));
    assert_eq!(page.num_scanned_files, 3);
    assert!(!page.reached_scan_cap);
}

#[tokio::test]
async fn test_pagination_cursor() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();

    // Fixed UUIDs for deterministic expectations
    let u1 = Uuid::from_u128(11);
    let u2 = Uuid::from_u128(22);
    let u3 = Uuid::from_u128(33);
    let u4 = Uuid::from_u128(44);
    let u5 = Uuid::from_u128(55);

    // Oldest to newest
    write_session_file(home, "2025-03-01T09-00-00", u1, 1).unwrap();
    write_session_file(home, "2025-03-02T09-00-00", u2, 1).unwrap();
    write_session_file(home, "2025-03-03T09-00-00", u3, 1).unwrap();
    write_session_file(home, "2025-03-04T09-00-00", u4, 1).unwrap();
    write_session_file(home, "2025-03-05T09-00-00", u5, 1).unwrap();

    // Page 1: newest 2 items
    let page1 = get_conversations(home, 2, None).await.unwrap();
    let p5 = home
        .join("sessions")
        .join("2025")
        .join("03")
        .join("05")
        .join(format!("rollout-2025-03-05T09-00-00-{u5}.jsonl"));
    let p4 = home
        .join("sessions")
        .join("2025")
        .join("03")
        .join("04")
        .join(format!("rollout-2025-03-04T09-00-00-{u4}.jsonl"));

    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.items[0].path, p5);
    assert_eq!(page1.items[1].path, p4);
    assert!(
        page1.items[0].head[0]["id"]
            .as_str()
            .unwrap()
            .contains(&u5.to_string())
    );

    let expected_cursor1: Cursor =
        serde_json::from_str(&format!("\"2025-03-04T09-00-00|{u4}\"")).unwrap();
    assert_eq!(page1.next_cursor, Some(expected_cursor1));
    assert_eq!(page1.num_scanned_files, 3);
    assert!(!page1.reached_scan_cap);

    // Page 2: next 2 items
    let page2 = get_conversations(home, 2, page1.next_cursor.as_ref())
        .await
        .unwrap();
    let p3 = home
        .join("sessions")
        .join("2025")
        .join("03")
        .join("03")
        .join(format!("rollout-2025-03-03T09-00-00-{u3}.jsonl"));
    let p2 = home
        .join("sessions")
        .join("2025")
        .join("03")
        .join("02")
        .join(format!("rollout-2025-03-02T09-00-00-{u2}.jsonl"));

    assert_eq!(page2.items.len(), 2);
    assert_eq!(page2.items[0].path, p3);
    assert_eq!(page2.items[1].path, p2);

    let expected_cursor2: Cursor =
        serde_json::from_str(&format!("\"2025-03-02T09-00-00|{u2}\"")).unwrap();
    assert_eq!(page2.next_cursor, Some(expected_cursor2));
    assert_eq!(page2.num_scanned_files, 5);
    assert!(!page2.reached_scan_cap);

    // Page 3: last item
    let page3 = get_conversations(home, 2, page2.next_cursor.as_ref())
        .await
        .unwrap();
    let p1 = home
        .join("sessions")
        .join("2025")
        .join("03")
        .join("01")
        .join(format!("rollout-2025-03-01T09-00-00-{u1}.jsonl"));

    assert_eq!(page3.items.len(), 1);
    assert_eq!(page3.items[0].path, p1);

    let expected_cursor3: Cursor =
        serde_json::from_str(&format!("\"2025-03-01T09-00-00|{u1}\"")).unwrap();
    assert_eq!(page3.next_cursor, Some(expected_cursor3));
    assert_eq!(page3.num_scanned_files, 5);
    assert!(!page3.reached_scan_cap);
}

#[tokio::test]
async fn test_get_conversation_contents() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();

    let uuid = Uuid::new_v4();
    let ts = "2025-04-01T10-30-00";
    write_session_file(home, ts, uuid, 2).unwrap();

    let page = get_conversations(home, 1, None).await.unwrap();
    let path = &page.items[0].path;

    let content = get_conversation(path).await.unwrap();

    // Verify page structure
    let expected_path = home
        .join("sessions")
        .join("2025")
        .join("04")
        .join("01")
        .join(format!("rollout-2025-04-01T10-30-00-{uuid}.jsonl"));

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].path, expected_path);

    // Verify head contains session ID
    assert!(
        page.items[0].head[0]["id"]
            .as_str()
            .unwrap()
            .contains(&uuid.to_string())
    );

    // Verify cursor
    let expected_cursor: Cursor = serde_json::from_str(&format!("\"{ts}|{uuid}\"")).unwrap();
    assert_eq!(page.next_cursor, Some(expected_cursor));
    assert_eq!(page.num_scanned_files, 1);
    assert!(!page.reached_scan_cap);

    // Verify file contains valid JSONL (each line parses as JSON)
    for line in content.lines() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(parsed.is_ok(), "Failed to parse line as JSON: {line}");
    }

    // Verify file contains the session ID
    assert!(content.contains(&uuid.to_string()));
}

#[tokio::test]
async fn test_stable_ordering_same_second_pagination() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();

    let ts = "2025-07-01T00-00-00";
    let u1 = Uuid::from_u128(1);
    let u2 = Uuid::from_u128(2);
    let u3 = Uuid::from_u128(3);

    write_session_file(home, ts, u1, 0).unwrap();
    write_session_file(home, ts, u2, 0).unwrap();
    write_session_file(home, ts, u3, 0).unwrap();

    // Page 1: All files have same timestamp, so ordered by UUID descending
    let page1 = get_conversations(home, 2, None).await.unwrap();

    let p3 = home
        .join("sessions")
        .join("2025")
        .join("07")
        .join("01")
        .join(format!("rollout-2025-07-01T00-00-00-{u3}.jsonl"));
    let p2 = home
        .join("sessions")
        .join("2025")
        .join("07")
        .join("01")
        .join(format!("rollout-2025-07-01T00-00-00-{u2}.jsonl"));

    // Verify ordering: highest UUID first (u3 > u2 > u1)
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.items[0].path, p3);
    assert_eq!(page1.items[1].path, p2);

    // Verify heads contain correct session IDs
    assert!(
        page1.items[0].head[0]["id"]
            .as_str()
            .unwrap()
            .contains(&u3.to_string())
    );
    assert!(
        page1.items[1].head[0]["id"]
            .as_str()
            .unwrap()
            .contains(&u2.to_string())
    );

    let expected_cursor1: Cursor = serde_json::from_str(&format!("\"{ts}|{u2}\"")).unwrap();
    assert_eq!(page1.next_cursor, Some(expected_cursor1));
    assert_eq!(page1.num_scanned_files, 3);
    assert!(!page1.reached_scan_cap);

    // Page 2: Last item
    let page2 = get_conversations(home, 2, page1.next_cursor.as_ref())
        .await
        .unwrap();
    let p1 = home
        .join("sessions")
        .join("2025")
        .join("07")
        .join("01")
        .join(format!("rollout-2025-07-01T00-00-00-{u1}.jsonl"));

    assert_eq!(page2.items.len(), 1);
    assert_eq!(page2.items[0].path, p1);
    assert!(
        page2.items[0].head[0]["id"]
            .as_str()
            .unwrap()
            .contains(&u1.to_string())
    );

    let expected_cursor2: Cursor = serde_json::from_str(&format!("\"{ts}|{u1}\"")).unwrap();
    assert_eq!(page2.next_cursor, Some(expected_cursor2));
    assert_eq!(page2.num_scanned_files, 3);
    assert!(!page2.reached_scan_cap);
}
