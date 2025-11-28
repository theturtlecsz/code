//! Process hardening for security-sensitive applications.
//!
//! This module provides pre-main hardening steps to protect the process from:
//! - Core dumps (sensitive data exposure)
//! - Debugger attachment (ptrace, gdb)
//! - Library injection (LD_PRELOAD, DYLD_*)
//!
//! # Usage
//!
//! Call `pre_main_hardening()` early in main() or use with `#[ctor::ctor]` for
//! pre-main execution.
//!
//! ```rust,ignore
//! fn main() {
//!     codex_process_hardening::pre_main_hardening();
//!     // ... rest of application
//! }
//! ```

/// Performs various process hardening steps appropriate for the current platform.
///
/// This function:
/// - Disables core dumps (sets RLIMIT_CORE to 0)
/// - Prevents debugger attachment (Linux: PR_SET_DUMPABLE, macOS: PT_DENY_ATTACH)
/// - Removes dangerous environment variables (LD_*, DYLD_*)
///
/// On failure, the process will exit with a non-zero exit code.
pub fn pre_main_hardening() {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pre_main_hardening_linux();

    #[cfg(target_os = "macos")]
    pre_main_hardening_macos();

    // On FreeBSD and OpenBSD, apply similar hardening to Linux/macOS:
    #[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
    pre_main_hardening_bsd();

    #[cfg(windows)]
    pre_main_hardening_windows();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
const PRCTL_FAILED_EXIT_CODE: i32 = 5;

#[cfg(target_os = "macos")]
const PTRACE_DENY_ATTACH_FAILED_EXIT_CODE: i32 = 6;

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
const SET_RLIMIT_CORE_FAILED_EXIT_CODE: i32 = 7;

#[cfg(any(target_os = "linux", target_os = "android"))]
fn pre_main_hardening_linux() {
    // Disable ptrace attach / mark process non-dumpable.
    let ret_code = unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) };
    if ret_code != 0 {
        eprintln!(
            "ERROR: prctl(PR_SET_DUMPABLE, 0) failed: {}",
            std::io::Error::last_os_error()
        );
        std::process::exit(PRCTL_FAILED_EXIT_CODE);
    }

    // For "defense in depth," set the core file size limit to 0.
    set_core_file_size_limit_to_zero();

    // Official Codex releases are MUSL-linked, which means that variables such
    // as LD_PRELOAD are ignored anyway, but just to be sure, clear them here.
    let ld_keys: Vec<String> = std::env::vars()
        .filter_map(|(key, _)| {
            if key.starts_with("LD_") {
                Some(key)
            } else {
                None
            }
        })
        .collect();

    for key in ld_keys {
        unsafe {
            std::env::remove_var(key);
        }
    }
}

#[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
fn pre_main_hardening_bsd() {
    // FreeBSD/OpenBSD: set RLIMIT_CORE to 0 and clear LD_* env vars
    set_core_file_size_limit_to_zero();

    let ld_keys: Vec<String> = std::env::vars()
        .filter_map(|(key, _)| {
            if key.starts_with("LD_") {
                Some(key)
            } else {
                None
            }
        })
        .collect();
    for key in ld_keys {
        unsafe {
            std::env::remove_var(key);
        }
    }
}

#[cfg(target_os = "macos")]
fn pre_main_hardening_macos() {
    // Prevent debuggers from attaching to this process.
    let ret_code = unsafe { libc::ptrace(libc::PT_DENY_ATTACH, 0, std::ptr::null_mut(), 0) };
    if ret_code == -1 {
        eprintln!(
            "ERROR: ptrace(PT_DENY_ATTACH) failed: {}",
            std::io::Error::last_os_error()
        );
        std::process::exit(PTRACE_DENY_ATTACH_FAILED_EXIT_CODE);
    }

    // Set the core file size limit to 0 to prevent core dumps.
    set_core_file_size_limit_to_zero();

    // Remove all DYLD_ environment variables, which can be used to subvert
    // library loading.
    let dyld_keys: Vec<String> = std::env::vars()
        .filter_map(|(key, _)| {
            if key.starts_with("DYLD_") {
                Some(key)
            } else {
                None
            }
        })
        .collect();

    for key in dyld_keys {
        unsafe {
            std::env::remove_var(key);
        }
    }
}

#[cfg(unix)]
fn set_core_file_size_limit_to_zero() {
    let rlim = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };

    let ret_code = unsafe { libc::setrlimit(libc::RLIMIT_CORE, &rlim) };
    if ret_code != 0 {
        eprintln!(
            "ERROR: setrlimit(RLIMIT_CORE) failed: {}",
            std::io::Error::last_os_error()
        );
        std::process::exit(SET_RLIMIT_CORE_FAILED_EXIT_CODE);
    }
}

#[cfg(windows)]
fn pre_main_hardening_windows() {
    // TODO: Perform the appropriate configuration for Windows.
    // Potential measures:
    // - SetProcessMitigationPolicy for various protections
    // - Disable debug privileges
    // - Enable DEP (Data Execution Prevention)
}
