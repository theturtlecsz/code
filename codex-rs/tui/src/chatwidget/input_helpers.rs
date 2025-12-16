//! Input helpers for text parsing and path handling.
//!
//! Contains pure functions for processing user input including:
//! - Image path detection
//! - URL decoding for file:// paths
//! - Terminal escape sequence handling
//!
//! Extracted from mod.rs as part of MAINT-11 to reduce duplication
//! and improve code organization.

/// Common image file extensions supported for drag-and-drop.
pub(crate) const IMAGE_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".svg", ".ico", ".tiff", ".tif",
];

/// Check if a path has an image file extension.
pub(crate) fn is_image_extension(path: &str) -> bool {
    let lower = path.to_lowercase();
    IMAGE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Check if text looks like a file path.
pub(crate) fn is_likely_file_path(text: &str) -> bool {
    text.starts_with("file://")
        || text.starts_with('/')
        || text.starts_with("~/")
        || text.starts_with("./")
}

/// Remove terminal escape backslashes from a path.
///
/// Terminals often add backslashes before special characters when pasting paths.
/// This removes common escapes like `\ ` -> ` `, `\(` -> `(`, `\)` -> `)`.
pub(crate) fn unescape_terminal_path(path: &str) -> String {
    path.replace("\\ ", " ")
        .replace("\\(", "(")
        .replace("\\)", ")")
}

/// Decode a file:// URL to a local path string.
///
/// Handles common URL-encoded characters like `%20` (space), `%28` (parenthesis), etc.
/// Returns the path portion without the `file://` prefix.
pub(crate) fn url_decode_file_path(url: &str) -> Option<String> {
    url.strip_prefix("file://").map(|s| {
        s.replace("%20", " ")
            .replace("%28", "(")
            .replace("%29", ")")
            .replace("%5B", "[")
            .replace("%5D", "]")
            .replace("%2C", ",")
            .replace("%27", "'")
            .replace("%26", "&")
            .replace("%23", "#")
            .replace("%40", "@")
            .replace("%2B", "+")
            .replace("%3D", "=")
            .replace("%24", "$")
            .replace("%21", "!")
            .replace("%2D", "-")
            .replace("%2E", ".")
    })
}

/// Parse a pasted string into a normalized path if it looks like a file path.
///
/// Returns `Some(path_string)` if the text appears to be a valid file path,
/// with terminal escapes and URL encoding removed.
pub(crate) fn normalize_pasted_path(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if !is_likely_file_path(trimmed) {
        return None;
    }
    let unescaped = unescape_terminal_path(trimmed);

    if let Some(decoded) = url_decode_file_path(&unescaped) {
        Some(decoded)
    } else {
        Some(unescaped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_extension() {
        assert!(is_image_extension("photo.png"));
        assert!(is_image_extension("Photo.PNG"));
        assert!(is_image_extension("/path/to/image.jpg"));
        assert!(is_image_extension("file.jpeg"));
        assert!(is_image_extension("image.webp"));
        assert!(!is_image_extension("document.pdf"));
        assert!(!is_image_extension("file.txt"));
        assert!(!is_image_extension("no_extension"));
    }

    #[test]
    fn test_is_likely_file_path() {
        assert!(is_likely_file_path("/home/user/file.txt"));
        assert!(is_likely_file_path("file:///home/user/file.txt"));
        assert!(is_likely_file_path("~/Documents/file.txt"));
        assert!(is_likely_file_path("./relative/path.txt"));
        assert!(!is_likely_file_path("just some text"));
        assert!(!is_likely_file_path("http://example.com"));
        assert!(!is_likely_file_path("relative/no/prefix.txt"));
    }

    #[test]
    fn test_unescape_terminal_path() {
        assert_eq!(
            unescape_terminal_path("/path/to/My\\ File.txt"),
            "/path/to/My File.txt"
        );
        assert_eq!(
            unescape_terminal_path("/path/to/File\\(1\\).txt"),
            "/path/to/File(1).txt"
        );
        assert_eq!(
            unescape_terminal_path("/simple/path.txt"),
            "/simple/path.txt"
        );
    }

    #[test]
    fn test_url_decode_file_path() {
        assert_eq!(
            url_decode_file_path("file:///path/to/My%20File.txt"),
            Some("/path/to/My File.txt".to_string())
        );
        assert_eq!(
            url_decode_file_path("file:///path/to/File%281%29.txt"),
            Some("/path/to/File(1).txt".to_string())
        );
        assert_eq!(
            url_decode_file_path("file:///path/with%5Bbrackets%5D.txt"),
            Some("/path/with[brackets].txt".to_string())
        );
        // Not a file:// URL
        assert_eq!(url_decode_file_path("/path/to/file.txt"), None);
        assert_eq!(url_decode_file_path("http://example.com"), None);
    }

    #[test]
    fn test_normalize_pasted_path() {
        // file:// URL
        assert_eq!(
            normalize_pasted_path("file:///path/to/My%20File.png"),
            Some("/path/to/My File.png".to_string())
        );
        // Escaped path
        assert_eq!(
            normalize_pasted_path("/path/to/My\\ File.png"),
            Some("/path/to/My File.png".to_string())
        );
        // Regular path
        assert_eq!(
            normalize_pasted_path("/path/to/file.png"),
            Some("/path/to/file.png".to_string())
        );
        // Home-relative path
        assert_eq!(
            normalize_pasted_path("~/Documents/image.jpg"),
            Some("~/Documents/image.jpg".to_string())
        );
        // Not a path
        assert_eq!(normalize_pasted_path("hello world"), None);
        // Whitespace trimmed
        assert_eq!(
            normalize_pasted_path("  /path/to/file.txt  "),
            Some("/path/to/file.txt".to_string())
        );
    }
}
