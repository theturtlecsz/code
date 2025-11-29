//! Keyboard hint utilities for consistent keybinding display.
//!
//! Provides platform-aware formatting for keyboard shortcuts in the TUI.

#![allow(dead_code)] // Module is new, utilities will be used in future footer refactor

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::style::{Style, Stylize};
use ratatui::text::Span;

// Platform-specific modifier prefixes
#[cfg(test)]
const ALT_PREFIX: &str = "⌥ + ";
#[cfg(all(not(test), target_os = "macos"))]
const ALT_PREFIX: &str = "⌥ + ";
#[cfg(all(not(test), not(target_os = "macos")))]
const ALT_PREFIX: &str = "alt + ";
const CTRL_PREFIX: &str = "ctrl + ";
const SHIFT_PREFIX: &str = "shift + ";

/// A keyboard binding with key and modifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct KeyBinding {
    key: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyBinding {
    /// Create a new key binding.
    pub(crate) const fn new(key: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    /// Check if this binding matches the given key event (press or repeat).
    pub fn is_press(&self, event: KeyEvent) -> bool {
        self.key == event.code
            && self.modifiers == event.modifiers
            && (event.kind == KeyEventKind::Press || event.kind == KeyEventKind::Repeat)
    }

    /// Get the key code.
    pub fn key(&self) -> KeyCode {
        self.key
    }

    /// Get the modifiers.
    pub fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }
}

/// Create a plain key binding (no modifiers).
pub(crate) const fn plain(key: KeyCode) -> KeyBinding {
    KeyBinding::new(key, KeyModifiers::NONE)
}

/// Create an Alt+key binding.
pub(crate) const fn alt(key: KeyCode) -> KeyBinding {
    KeyBinding::new(key, KeyModifiers::ALT)
}

/// Create a Shift+key binding.
pub(crate) const fn shift(key: KeyCode) -> KeyBinding {
    KeyBinding::new(key, KeyModifiers::SHIFT)
}

/// Create a Ctrl+key binding.
pub(crate) const fn ctrl(key: KeyCode) -> KeyBinding {
    KeyBinding::new(key, KeyModifiers::CONTROL)
}

fn modifiers_to_string(modifiers: KeyModifiers) -> String {
    let mut result = String::new();
    if modifiers.contains(KeyModifiers::CONTROL) {
        result.push_str(CTRL_PREFIX);
    }
    if modifiers.contains(KeyModifiers::SHIFT) {
        result.push_str(SHIFT_PREFIX);
    }
    if modifiers.contains(KeyModifiers::ALT) {
        result.push_str(ALT_PREFIX);
    }
    result
}

impl From<KeyBinding> for Span<'static> {
    fn from(binding: KeyBinding) -> Self {
        (&binding).into()
    }
}

impl From<&KeyBinding> for Span<'static> {
    fn from(binding: &KeyBinding) -> Self {
        let KeyBinding { key, modifiers } = binding;
        let modifiers = modifiers_to_string(*modifiers);
        let key = match key {
            KeyCode::Enter => "enter".to_string(),
            KeyCode::Esc => "esc".to_string(),
            KeyCode::Tab => "tab".to_string(),
            KeyCode::Backspace => "backspace".to_string(),
            KeyCode::Delete => "delete".to_string(),
            KeyCode::Up => "↑".to_string(),
            KeyCode::Down => "↓".to_string(),
            KeyCode::Left => "←".to_string(),
            KeyCode::Right => "→".to_string(),
            KeyCode::PageUp => "pgup".to_string(),
            KeyCode::PageDown => "pgdn".to_string(),
            KeyCode::Home => "home".to_string(),
            KeyCode::End => "end".to_string(),
            _ => format!("{key}").to_ascii_lowercase(),
        };
        Span::styled(format!("{modifiers}{key}"), key_hint_style())
    }
}

fn key_hint_style() -> Style {
    Style::default().dim()
}

/// Check if modifiers include Ctrl or Alt (but not AltGr).
pub(crate) fn has_ctrl_or_alt(mods: KeyModifiers) -> bool {
    (mods.contains(KeyModifiers::CONTROL) || mods.contains(KeyModifiers::ALT)) && !is_altgr(mods)
}

/// Check if modifiers represent AltGr (Windows-specific: Ctrl+Alt).
#[cfg(windows)]
#[inline]
pub(crate) fn is_altgr(mods: KeyModifiers) -> bool {
    mods.contains(KeyModifiers::ALT) && mods.contains(KeyModifiers::CONTROL)
}

#[cfg(not(windows))]
#[inline]
pub(crate) fn is_altgr(_mods: KeyModifiers) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_binding_plain() {
        let binding = plain(KeyCode::Char('a'));
        let span: Span = binding.into();
        assert_eq!(span.content.as_ref(), "a");
    }

    #[test]
    fn key_binding_ctrl() {
        let binding = ctrl(KeyCode::Char('c'));
        let span: Span = binding.into();
        assert_eq!(span.content.as_ref(), "ctrl + c");
    }

    #[test]
    fn key_binding_shift() {
        let binding = shift(KeyCode::Enter);
        let span: Span = binding.into();
        assert_eq!(span.content.as_ref(), "shift + enter");
    }

    #[test]
    fn key_binding_special_keys() {
        assert_eq!(Span::from(plain(KeyCode::Up)).content.as_ref(), "↑");
        assert_eq!(
            Span::from(plain(KeyCode::PageDown)).content.as_ref(),
            "pgdn"
        );
    }

    #[test]
    fn is_press_matches_correctly() {
        let binding = ctrl(KeyCode::Char('c'));
        let event = KeyEvent::new_with_kind(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        );
        assert!(binding.is_press(event));

        let wrong_key = KeyEvent::new_with_kind(
            KeyCode::Char('x'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        );
        assert!(!binding.is_press(wrong_key));
    }

    #[test]
    fn has_ctrl_or_alt_detection() {
        assert!(has_ctrl_or_alt(KeyModifiers::CONTROL));
        assert!(has_ctrl_or_alt(KeyModifiers::ALT));
        assert!(!has_ctrl_or_alt(KeyModifiers::SHIFT));
        assert!(!has_ctrl_or_alt(KeyModifiers::NONE));
    }
}
