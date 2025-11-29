//! UTF-8 boundary-safe string truncation utilities.
//!
//! These functions safely truncate strings to a byte budget while ensuring
//! the result remains valid UTF-8 by respecting character boundaries.

/// Truncate a `&str` to a byte budget at a character boundary (prefix).
///
/// Returns the longest prefix of `s` that fits within `max_bytes` bytes
/// while ending at a valid UTF-8 character boundary.
///
/// # Examples
///
/// ```
/// use codex_utils_string::take_bytes_at_char_boundary;
///
/// // ASCII strings truncate directly
/// assert_eq!(take_bytes_at_char_boundary("hello world", 5), "hello");
///
/// // Multi-byte characters are not split
/// assert_eq!(take_bytes_at_char_boundary("héllo", 2), "h"); // é is 2 bytes
/// assert_eq!(take_bytes_at_char_boundary("héllo", 3), "hé");
///
/// // Emoji (4 bytes each) are preserved or excluded
/// assert_eq!(take_bytes_at_char_boundary("😀abc", 3), ""); // 😀 is 4 bytes
/// assert_eq!(take_bytes_at_char_boundary("😀abc", 4), "😀");
/// ```
#[inline]
pub fn take_bytes_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut last_ok = 0;
    for (i, ch) in s.char_indices() {
        let next_byte = i + ch.len_utf8();
        if next_byte > max_bytes {
            break;
        }
        last_ok = next_byte;
    }
    &s[..last_ok]
}

/// Take a suffix of a `&str` within a byte budget at a character boundary.
///
/// Returns the longest suffix of `s` that fits within `max_bytes` bytes
/// while starting at a valid UTF-8 character boundary.
///
/// # Examples
///
/// ```
/// use codex_utils_string::take_last_bytes_at_char_boundary;
///
/// // ASCII strings truncate directly
/// assert_eq!(take_last_bytes_at_char_boundary("hello world", 5), "world");
///
/// // Multi-byte characters are not split
/// assert_eq!(take_last_bytes_at_char_boundary("helloé", 2), "é"); // é is 2 bytes
/// assert_eq!(take_last_bytes_at_char_boundary("helloé", 1), ""); // Can't fit é
///
/// // Emoji are preserved or excluded
/// assert_eq!(take_last_bytes_at_char_boundary("abc😀", 4), "😀");
/// assert_eq!(take_last_bytes_at_char_boundary("abc😀", 3), "");
/// ```
#[inline]
pub fn take_last_bytes_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut start = s.len();
    let mut used = 0usize;
    for (i, ch) in s.char_indices().rev() {
        let char_bytes = ch.len_utf8();
        if used + char_bytes > max_bytes {
            break;
        }
        start = i;
        used += char_bytes;
        if start == 0 {
            break;
        }
    }
    &s[start..]
}

/// Check if a byte slice is valid UTF-8.
///
/// # Examples
///
/// ```
/// use codex_utils_string::is_valid_utf8;
///
/// assert!(is_valid_utf8(b"hello"));
/// assert!(is_valid_utf8("émoji 😀".as_bytes()));
/// assert!(!is_valid_utf8(&[0xff, 0xfe])); // Invalid UTF-8
/// ```
#[inline]
pub fn is_valid_utf8(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok()
}

/// Decode bytes as UTF-8, replacing invalid sequences with the replacement character.
///
/// This is a thin wrapper around `String::from_utf8_lossy` for convenience.
#[inline]
pub fn decode_utf8_lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_truncation_ascii() {
        assert_eq!(take_bytes_at_char_boundary("hello world", 5), "hello");
        assert_eq!(take_bytes_at_char_boundary("hello", 10), "hello");
        assert_eq!(take_bytes_at_char_boundary("hello", 0), "");
    }

    #[test]
    fn prefix_truncation_multibyte() {
        // é is 2 bytes in UTF-8
        assert_eq!(take_bytes_at_char_boundary("héllo", 1), "h");
        assert_eq!(take_bytes_at_char_boundary("héllo", 2), "h"); // Can't fit é
        assert_eq!(take_bytes_at_char_boundary("héllo", 3), "hé");

        // 😀 is 4 bytes
        assert_eq!(take_bytes_at_char_boundary("😀abc", 3), "");
        assert_eq!(take_bytes_at_char_boundary("😀abc", 4), "😀");
        assert_eq!(take_bytes_at_char_boundary("😀abc", 5), "😀a");
    }

    #[test]
    fn suffix_truncation_ascii() {
        assert_eq!(take_last_bytes_at_char_boundary("hello world", 5), "world");
        assert_eq!(take_last_bytes_at_char_boundary("hello", 10), "hello");
        assert_eq!(take_last_bytes_at_char_boundary("hello", 0), "");
    }

    #[test]
    fn suffix_truncation_multibyte() {
        // é is 2 bytes
        assert_eq!(take_last_bytes_at_char_boundary("helloé", 2), "é");
        assert_eq!(take_last_bytes_at_char_boundary("helloé", 1), "");
        assert_eq!(take_last_bytes_at_char_boundary("helloé", 3), "oé");

        // 😀 is 4 bytes
        assert_eq!(take_last_bytes_at_char_boundary("abc😀", 4), "😀");
        assert_eq!(take_last_bytes_at_char_boundary("abc😀", 3), "");
        assert_eq!(take_last_bytes_at_char_boundary("abc😀", 5), "c😀");
    }

    #[test]
    fn edge_cases_empty_string() {
        assert_eq!(take_bytes_at_char_boundary("", 10), "");
        assert_eq!(take_last_bytes_at_char_boundary("", 10), "");
    }

    #[test]
    fn utf8_validation() {
        assert!(is_valid_utf8(b"hello"));
        assert!(is_valid_utf8("🎉".as_bytes()));
        assert!(!is_valid_utf8(&[0xff, 0xfe]));
        assert!(!is_valid_utf8(&[0x80])); // Continuation byte without start
    }

    #[test]
    fn lossy_decode() {
        assert_eq!(decode_utf8_lossy(b"hello"), "hello");
        assert_eq!(decode_utf8_lossy(&[0xff, 0xfe]), "��");
    }
}
