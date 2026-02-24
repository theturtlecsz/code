use std::sync::OnceLock;

static TERMINAL: OnceLock<String> = OnceLock::new();

pub fn user_agent() -> String {
    TERMINAL.get_or_init(detect_terminal).to_string()
}

/// Sanitize a header value to be used in a User-Agent string.
///
/// This function replaces any characters that are not allowed in a User-Agent string with an underscore.
///
/// # Arguments
///
/// * `value` - The value to sanitize.
fn is_valid_header_value_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/'
}

fn sanitize_header_value(value: String) -> String {
    value.replace(|c| !is_valid_header_value_char(c), "_")
}

fn detect_terminal() -> String {
    let result = if let Ok(tp) = std::env::var("TERM_PROGRAM") {
        let tp_trimmed = tp.trim();
        if !tp_trimmed.is_empty() {
            let ver = std::env::var("TERM_PROGRAM_VERSION").ok();
            match ver {
                Some(v) if !v.trim().is_empty() => format!("{tp}/{v}"),
                _ => tp,
            }
        } else {
            // fall through to other detectors
            String::new()
        }
    } else {
        String::new()
    };

    let detected = if !result.is_empty() {
        result
    } else if let Ok(v) = std::env::var("WEZTERM_VERSION") {
        if !v.trim().is_empty() {
            format!("WezTerm/{v}")
        } else {
            "WezTerm".to_string()
        }
    } else if std::env::var("KITTY_WINDOW_ID").is_ok()
        || std::env::var("TERM")
            .map(|t| t.contains("kitty"))
            .unwrap_or(false)
    {
        "kitty".to_string()
    } else if std::env::var("ALACRITTY_SOCKET").is_ok()
        || std::env::var("TERM")
            .map(|t| t == "alacritty")
            .unwrap_or(false)
    {
        "Alacritty".to_string()
    } else if let Ok(v) = std::env::var("KONSOLE_VERSION") {
        if !v.trim().is_empty() {
            format!("Konsole/{v}")
        } else {
            "Konsole".to_string()
        }
    } else if std::env::var("GNOME_TERMINAL_SCREEN").is_ok() {
        "gnome-terminal".to_string()
    } else if let Ok(v) = std::env::var("VTE_VERSION") {
        if !v.trim().is_empty() {
            format!("VTE/{v}")
        } else {
            "VTE".to_string()
        }
    } else if std::env::var("WT_SESSION").is_ok() {
        "WindowsTerminal".to_string()
    } else {
        std::env::var("TERM").unwrap_or_else(|_| "unknown".to_string())
    };

    sanitize_header_value(detected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_header_value_valid() {
        let input = "valid-header_value.123/456";
        assert_eq!(sanitize_header_value(input.to_string()), input);
    }

    #[test]
    fn test_sanitize_header_value_invalid() {
        let input = "invalid header value@#$";
        let expected = "invalid_header_value___";
        assert_eq!(sanitize_header_value(input.to_string()), expected);
    }

    #[test]
    fn test_sanitize_header_value_empty() {
        let input = "";
        assert_eq!(sanitize_header_value(input.to_string()), "");
    }

    #[test]
    fn test_sanitize_header_value_mixed() {
        let input = "Term/1.0 (My OS)";
        let expected = "Term/1.0__My_OS_";
        assert_eq!(sanitize_header_value(input.to_string()), expected);
    }

    #[test]
    fn test_is_valid_header_value_char() {
        assert!(is_valid_header_value_char('a'));
        assert!(is_valid_header_value_char('Z'));
        assert!(is_valid_header_value_char('0'));
        assert!(is_valid_header_value_char('-'));
        assert!(is_valid_header_value_char('_'));
        assert!(is_valid_header_value_char('.'));
        assert!(is_valid_header_value_char('/'));

        assert!(!is_valid_header_value_char(' '));
        assert!(!is_valid_header_value_char('@'));
        assert!(!is_valid_header_value_char(':'));
        assert!(!is_valid_header_value_char('\n'));
    }
}
