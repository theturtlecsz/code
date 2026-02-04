mod osc9;
mod windows_toast;

use std::env;
use std::io;

use crate::compat::env::is_wsl;
use osc9::Osc9Backend;
use windows_toast::WindowsToastBackend;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationBackendKind {
    Osc9,
    WindowsToast,
}

#[derive(Debug)]
pub enum DesktopNotificationBackend {
    Osc9(Osc9Backend),
    WindowsToast(WindowsToastBackend),
}

impl DesktopNotificationBackend {
    pub fn osc9() -> Self {
        Self::Osc9(Osc9Backend)
    }

    pub fn windows_toast() -> Self {
        Self::WindowsToast(WindowsToastBackend::default())
    }

    pub fn kind(&self) -> NotificationBackendKind {
        match self {
            DesktopNotificationBackend::Osc9(_) => NotificationBackendKind::Osc9,
            DesktopNotificationBackend::WindowsToast(_) => NotificationBackendKind::WindowsToast,
        }
    }

    pub fn notify(&mut self, message: &str) -> io::Result<()> {
        match self {
            DesktopNotificationBackend::Osc9(backend) => backend.notify(message),
            DesktopNotificationBackend::WindowsToast(backend) => backend.notify(message),
        }
    }
}

pub fn detect_backend() -> DesktopNotificationBackend {
    if should_use_windows_toasts() {
        tracing::info!(
            "Windows Terminal session detected under WSL; using Windows toast notifications"
        );
        DesktopNotificationBackend::windows_toast()
    } else {
        DesktopNotificationBackend::osc9()
    }
}

fn should_use_windows_toasts() -> bool {
    is_wsl() && env::var_os("WT_SESSION").is_some()
}
