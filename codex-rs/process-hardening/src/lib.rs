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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that pre_main_hardening runs without panic on the current platform.
    /// This is an integration test that verifies the syscalls succeed.
    #[test]
    fn test_pre_main_hardening_succeeds() {
        // Should not panic or exit
        pre_main_hardening();
    }

    /// Verify RLIMIT_CORE is set to 0 after hardening.
    #[cfg(unix)]
    #[test]
    fn test_rlimit_core_is_zero() {
        pre_main_hardening();

        let mut rlim = libc::rlimit {
            rlim_cur: u64::MAX,
            rlim_max: u64::MAX,
        };

        let ret = unsafe { libc::getrlimit(libc::RLIMIT_CORE, &mut rlim) };
        assert_eq!(ret, 0, "getrlimit should succeed");
        assert_eq!(rlim.rlim_cur, 0, "RLIMIT_CORE soft limit should be 0");
        assert_eq!(rlim.rlim_max, 0, "RLIMIT_CORE hard limit should be 0");
    }

    /// Verify LD_* environment variables are cleared on Linux.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn test_ld_env_vars_cleared() {
        // Set a test LD_ variable
        unsafe {
            std::env::set_var("LD_TEST_VAR", "should_be_removed");
        }

        pre_main_hardening();

        assert!(
            std::env::var("LD_TEST_VAR").is_err(),
            "LD_TEST_VAR should be removed"
        );
    }

    /// Verify DYLD_* environment variables are cleared on macOS.
    #[cfg(target_os = "macos")]
    #[test]
    fn test_dyld_env_vars_cleared() {
        // Set a test DYLD_ variable
        unsafe {
            std::env::set_var("DYLD_TEST_VAR", "should_be_removed");
        }

        pre_main_hardening();

        assert!(
            std::env::var("DYLD_TEST_VAR").is_err(),
            "DYLD_TEST_VAR should be removed"
        );
    }

    /// Verify process is non-dumpable on Linux after hardening.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn test_process_non_dumpable() {
        pre_main_hardening();

        let dumpable = unsafe { libc::prctl(libc::PR_GET_DUMPABLE, 0, 0, 0, 0) };
        assert_eq!(dumpable, 0, "Process should be non-dumpable");
    }
}
