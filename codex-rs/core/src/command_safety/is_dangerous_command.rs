//! Dangerous command detection for shell commands.
//!
//! This module provides heuristics to identify commands that are potentially
//! dangerous (e.g., `git reset --hard`, `rm -rf`) and should prompt for user
//! approval even in permissive sandbox modes.

use crate::bash::try_parse_bash;
use crate::bash::try_parse_word_only_commands_sequence;
use std::path::Path;

/// Returns true if the command looks like a potentially dangerous operation
/// that should prompt for user approval.
///
/// This is a heuristic check that looks for commands known to be destructive
/// (e.g., `git reset`, `rm -rf`, etc.) even when running in a permissive
/// sandbox mode like `DangerFullAccess`.
pub fn command_might_be_dangerous(command: &[String]) -> bool {
    if is_dangerous_to_call_with_exec(command) {
        return true;
    }

    // Support `bash -lc "<script>"` or `zsh -lc "<script>"` where any part
    // of the script might contain a dangerous command.
    if let Some(all_commands) = parse_shell_lc_plain_commands(command)
        && all_commands
            .iter()
            .any(|cmd| is_dangerous_to_call_with_exec(cmd))
    {
        return true;
    }

    false
}

/// Parses a shell invocation like `bash -lc "command1 && command2"` and returns
/// the individual commands if the invocation is in that form.
///
/// Returns `None` if the command is not a `bash -lc` or `zsh -lc` style invocation,
/// or if parsing fails.
fn parse_shell_lc_plain_commands(command: &[String]) -> Option<Vec<Vec<String>>> {
    let [shell, flag, script] = command else {
        return None;
    };

    // Check if first arg is bash or zsh (including full paths)
    let shell_basename = Path::new(shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(shell)
        .to_lowercase();

    if !matches!(
        shell_basename.as_str(),
        "bash" | "zsh" | "bash.exe" | "zsh.exe"
    ) {
        return None;
    }

    if flag != "-lc" {
        return None;
    }

    // Parse the script using tree-sitter-bash
    let tree = try_parse_bash(script)?;
    try_parse_word_only_commands_sequence(&tree, script)
}

/// Returns true if directly executing this command could be dangerous.
fn is_dangerous_to_call_with_exec(command: &[String]) -> bool {
    let cmd0 = command.first().map(String::as_str);

    match cmd0 {
        // Git destructive operations
        Some(cmd) if cmd.ends_with("git") || cmd.ends_with("/git") => {
            // Heuristic: Check if any argument is a destructive git subcommand.
            // This might trigger false positives (e.g. `git commit -m "reset"`),
            // but ensures we catch `git --no-pager reset`.
            command
                .iter()
                .skip(1)
                .any(|arg| matches!(arg.as_str(), "reset" | "rm" | "clean"))
        }

        // rm with force or recursive flags
        Some("rm") => {
            command.iter().skip(1).any(|arg| {
                if arg.starts_with("--") {
                    matches!(arg.as_str(), "--recursive" | "--force")
                } else if arg.starts_with("-") {
                    // Check for combined flags like -rf, -vfr
                    arg.chars().any(|c| matches!(c, 'r' | 'R' | 'f'))
                } else {
                    false
                }
            })
        }

        // Destructive disk operations
        Some("dd") | Some("mkfs") => true,
        Some(cmd) if cmd.starts_with("mkfs.") => true,

        // For `sudo <cmd>`, check the inner command
        Some("sudo") => is_dangerous_to_call_with_exec(&command[1..]),

        // Everything else is not flagged as dangerous
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec_str(items: &[&str]) -> Vec<String> {
        items.iter().map(std::string::ToString::to_string).collect()
    }

    #[test]
    fn git_reset_is_dangerous() {
        assert!(command_might_be_dangerous(&vec_str(&["git", "reset"])));
    }

    #[test]
    fn bash_git_reset_is_dangerous() {
        assert!(command_might_be_dangerous(&vec_str(&[
            "bash",
            "-lc",
            "git reset --hard"
        ])));
    }

    #[test]
    fn zsh_git_reset_is_dangerous() {
        assert!(command_might_be_dangerous(&vec_str(&[
            "zsh",
            "-lc",
            "git reset --hard"
        ])));
    }

    #[test]
    fn git_status_is_not_dangerous() {
        assert!(!command_might_be_dangerous(&vec_str(&["git", "status"])));
    }

    #[test]
    fn bash_git_status_is_not_dangerous() {
        assert!(!command_might_be_dangerous(&vec_str(&[
            "bash",
            "-lc",
            "git status"
        ])));
    }

    #[test]
    fn sudo_git_reset_is_dangerous() {
        assert!(command_might_be_dangerous(&vec_str(&[
            "sudo", "git", "reset", "--hard"
        ])));
    }

    #[test]
    fn usr_bin_git_is_dangerous() {
        assert!(command_might_be_dangerous(&vec_str(&[
            "/usr/bin/git",
            "reset",
            "--hard"
        ])));
    }

    #[test]
    fn git_with_flags_reset_is_dangerous() {
        assert!(command_might_be_dangerous(&vec_str(&[
            "git",
            "--no-pager",
            "reset",
            "--hard"
        ])));
    }

    #[test]
    fn rm_rf_is_dangerous() {
        assert!(command_might_be_dangerous(&vec_str(&["rm", "-rf", "/"])));
    }

    #[test]
    fn rm_f_is_dangerous() {
        assert!(command_might_be_dangerous(&vec_str(&["rm", "-f", "/"])));
    }

    #[test]
    fn rm_v_rf_is_dangerous() {
        assert!(command_might_be_dangerous(&vec_str(&[
            "rm", "-v", "-rf", "/"
        ])));
    }

    #[test]
    fn rm_combined_flags_is_dangerous() {
        assert!(command_might_be_dangerous(&vec_str(&["rm", "-vfr", "/"])));
    }

    #[test]
    fn dd_is_dangerous() {
        assert!(command_might_be_dangerous(&vec_str(&[
            "dd",
            "if=/dev/zero",
            "of=/dev/sda"
        ])));
    }

    #[test]
    fn mkfs_is_dangerous() {
        assert!(command_might_be_dangerous(&vec_str(&[
            "mkfs.ext4",
            "/dev/sda1"
        ])));
    }
}
