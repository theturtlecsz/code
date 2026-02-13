//! SPEC-PM-004: PM overlay keyboard handlers
//!
//! Follows the pattern established by limits_handlers.rs and help_handlers.rs.

use super::ChatWidget;
use crossterm::event::{KeyCode, KeyEvent};

pub(super) fn handle_pm_key(chat: &mut ChatWidget<'_>, key_event: KeyEvent) -> bool {
    let Some(ref overlay) = chat.pm.overlay else {
        return false;
    };

    match key_event.code {
        KeyCode::Esc => {
            chat.pm.overlay = None;
            chat.request_redraw();
            true
        }
        KeyCode::Up => {
            let sel = overlay.selected();
            if sel > 0 {
                overlay.set_selected(sel - 1);
                // Scroll up if selection is above viewport
                let scroll = overlay.scroll() as usize;
                if sel - 1 < scroll {
                    overlay.set_scroll((sel - 1) as u16);
                }
                chat.request_redraw();
            }
            true
        }
        KeyCode::Down => {
            let sel = overlay.selected();
            let max = overlay.visible_count().saturating_sub(1);
            if sel < max {
                overlay.set_selected(sel + 1);
                // Scroll down if selection is below viewport
                let scroll = overlay.scroll() as usize;
                let visible = overlay.visible_rows().max(1) as usize;
                if sel + 1 >= scroll + visible {
                    overlay.set_scroll((sel + 1).saturating_sub(visible.saturating_sub(1)) as u16);
                }
                chat.request_redraw();
            }
            true
        }
        KeyCode::Right => {
            let sel = overlay.selected();
            if overlay.expand_visible(sel) {
                chat.request_redraw();
            }
            true
        }
        KeyCode::Left => {
            let sel = overlay.selected();
            // If expanded, collapse; otherwise jump to parent
            if overlay.is_expanded_visible(sel) {
                overlay.collapse_visible(sel);
                chat.request_redraw();
            } else if let Some(parent) = overlay.parent_of_visible(sel) {
                overlay.set_selected(parent);
                // Adjust scroll if needed
                let scroll = overlay.scroll() as usize;
                if parent < scroll {
                    overlay.set_scroll(parent as u16);
                }
                chat.request_redraw();
            }
            true
        }
        KeyCode::PageUp => {
            let step = overlay.visible_rows() as usize;
            let sel = overlay.selected();
            let new_sel = sel.saturating_sub(step);
            overlay.set_selected(new_sel);
            let scroll = overlay.scroll() as usize;
            if new_sel < scroll {
                overlay.set_scroll(new_sel as u16);
            }
            chat.request_redraw();
            true
        }
        KeyCode::PageDown => {
            let step = overlay.visible_rows() as usize;
            let sel = overlay.selected();
            let max = overlay.visible_count().saturating_sub(1);
            let new_sel = (sel + step).min(max);
            overlay.set_selected(new_sel);
            let scroll = overlay.scroll() as usize;
            let visible = overlay.visible_rows().max(1) as usize;
            if new_sel >= scroll + visible {
                overlay.set_scroll(new_sel.saturating_sub(visible.saturating_sub(1)) as u16);
            }
            chat.request_redraw();
            true
        }
        KeyCode::Home => {
            overlay.set_selected(0);
            overlay.set_scroll(0);
            chat.request_redraw();
            true
        }
        KeyCode::End => {
            let max = overlay.visible_count().saturating_sub(1);
            overlay.set_selected(max);
            let visible = overlay.visible_rows() as usize;
            let scroll = max.saturating_sub(visible.saturating_sub(1));
            overlay.set_scroll(scroll as u16);
            chat.request_redraw();
            true
        }
        _ => false,
    }
}
