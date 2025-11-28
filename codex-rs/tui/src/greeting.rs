#[cfg(not(test))]
use chrono::Local;
#[cfg(not(test))]
use chrono::Timelike;

/// Build a time-aware placeholder like
/// "What can I code for you this morning?".
///
/// In test mode (cfg(test)), returns a fixed greeting to ensure
/// snapshot tests are deterministic regardless of when they run.
#[cfg(not(test))]
pub(crate) fn greeting_placeholder() -> String {
    let hour = Local::now().hour();
    // Custom mapping: show "today" for 10:00–13:59 local time.
    let when = if (10..=13).contains(&hour) {
        "today"
    } else if (5..=9).contains(&hour) {
        "this morning"
    } else if (14..=16).contains(&hour) {
        "this afternoon"
    } else if (17..=20).contains(&hour) {
        "this evening"
    } else {
        // Late night and very early hours
        "tonight"
    };
    format!("What can I code for you {when}?")
}

/// Test version: returns a fixed greeting for deterministic snapshots
#[cfg(test)]
pub(crate) fn greeting_placeholder() -> String {
    "What can I code for you today?".to_string()
}
