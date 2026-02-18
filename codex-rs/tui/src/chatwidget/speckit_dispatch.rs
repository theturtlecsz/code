//! Spec-kit command dispatch for ChatWidget.
//!
//! Extracted from mod.rs to reduce file size and merge conflict risk.
//! Contains: branch commands, spec-ops, spec-auto, merge, project,
//! guardrail evaluation, and SpecKitContext trait implementation.

use super::*;

impl ChatWidget<'_> {
    // Review handlers extracted to review_handlers.rs (MAINT-11 Phase 7)

    /// Handle `/branch [task]` command. Creates a worktree under `.code/branches`,
    /// optionally copies current uncommitted changes, then switches the session cwd
    /// into the worktree. If `task` is non-empty, submits it immediately.
    pub(crate) fn handle_branch_command(&mut self, args: String) {
        if Self::is_branch_worktree_path(&self.config.cwd) {
            self.history_push(crate::history_cell::new_error_event(
                "`/branch` — already inside a branch worktree; switch to the repo root before creating another branch."
                    .to_string(),
            ));
            self.request_redraw();
            return;
        }
        let args_trim = args.trim().to_string();
        let cwd = self.config.cwd.clone();
        let tx = self.app_event_tx.clone();
        // Add a quick notice into history, include task preview if provided
        if args_trim.is_empty() {
            self.insert_background_event_with_placement(
                "Creating branch worktree...".to_string(),
                BackgroundPlacement::BeforeNextOutput,
            );
        } else {
            self.insert_background_event_with_placement(
                format!("Creating branch worktree... Task: {}", args_trim),
                BackgroundPlacement::BeforeNextOutput,
            );
        }
        self.request_redraw();

        tokio::spawn(async move {
            use tokio::process::Command;
            // Resolve git root
            let git_root = match codex_core::git_worktree::get_git_root_from(&cwd).await {
                Ok(p) => p,
                Err(e) => {
                    tx.send_background_event(format!("`/branch` — not a git repo: {}", e));
                    return;
                }
            };
            // Determine branch name
            let task_opt = if args.trim().is_empty() {
                None
            } else {
                Some(args.trim())
            };
            let branch_name = codex_core::git_worktree::generate_branch_name_from_task(task_opt);
            // Create worktree
            let (worktree, used_branch) =
                match codex_core::git_worktree::setup_worktree(&git_root, &branch_name).await {
                    Ok((p, b)) => (p, b),
                    Err(e) => {
                        tx.send_background_event(format!(
                            "`/branch` — failed to create worktree: {}",
                            e
                        ));
                        return;
                    }
                };
            // Copy uncommitted changes from the source root into the new worktree
            let copied =
                match codex_core::git_worktree::copy_uncommitted_to_worktree(&git_root, &worktree)
                    .await
                {
                    Ok(n) => n,
                    Err(e) => {
                        tx.send_background_event(format!(
                            "`/branch` — failed to copy changes: {}",
                            e
                        ));
                        // Still switch to the branch even if copy fails
                        0
                    }
                };

            // Attempt to set upstream for the new branch to match the source branch's upstream,
            // falling back to origin/<default> when available. Also ensure origin/HEAD is set.
            let mut _upstream_msg: Option<String> = None;
            // Discover source branch upstream like 'origin/main'
            let src_upstream = Command::new("git")
                .current_dir(&git_root)
                .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
                .output()
                .await
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| {
                    let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    if s.is_empty() { None } else { Some(s) }
                });
            // Ensure origin/HEAD points at the remote default, if origin exists.
            let _ = Command::new("git")
                .current_dir(&git_root)
                .args(["remote", "set-head", "origin", "-a"])
                .output()
                .await;
            // Compute fallback remote default
            let fallback_remote = codex_core::git_worktree::detect_default_branch(&git_root)
                .await
                .map(|d| format!("origin/{}", d));
            let target_upstream = src_upstream.clone().or(fallback_remote);
            if let Some(up) = target_upstream {
                let set = Command::new("git")
                    .current_dir(&worktree)
                    .args([
                        "branch",
                        "--set-upstream-to",
                        up.as_str(),
                        used_branch.as_str(),
                    ])
                    .output()
                    .await;
                if let Ok(o) = set {
                    if o.status.success() {
                        _upstream_msg =
                            Some(format!("Set upstream for '{}' to {}", used_branch, up));
                    } else {
                        let e = String::from_utf8_lossy(&o.stderr).trim().to_string();
                        if !e.is_empty() {
                            _upstream_msg = Some(format!("Upstream not set ({}).", e));
                        }
                    }
                }
            }

            // Build clean multi-line output as a BackgroundEvent (not streaming Answer)
            let msg = if let Some(task_text) = task_opt {
                format!(
                    "Created worktree '{used}'\n  Path: {path}\n  Copied {copied} changed files\n  Task: {task}\n  Starting task...",
                    used = used_branch,
                    path = worktree.display(),
                    copied = copied,
                    task = task_text
                )
            } else {
                format!(
                    "Created worktree '{used}'\n  Path: {path}\n  Copied {copied} changed files\n  Type your task when ready.",
                    used = used_branch,
                    path = worktree.display(),
                    copied = copied
                )
            };
            {
                tx.send_background_event(msg);
            }

            // Switch cwd and optionally submit the task
            // Prefix the auto-submitted task so it's obvious it started in the new branch
            let initial_prompt = task_opt.map(|s| format!("[branch created] {}", s));
            tx.send(AppEvent::SwitchCwd(worktree, initial_prompt));
        });
    }

    // === FORK-SPECIFIC: spec-kit guardrail command handler ===
    // Upstream: Does not have spec-ops commands
    // Preserve: This entire function during rebases
    pub(crate) fn handle_spec_ops_command(
        &mut self,
        command: SlashCommand,
        raw_args: String,
        hal_override: Option<HalMode>,
    ) {
        spec_kit::handle_guardrail(self, command, raw_args, hal_override);
    }

    pub(crate) fn handle_spec_status_command(&mut self, raw_args: String) {
        spec_kit::handle_spec_status(self, raw_args);
    }
    // === END FORK-SPECIFIC: handle_spec_ops_command ===

    // === FORK-SPECIFIC: spec-kit consensus lookup ===
    // Upstream: Does not have /spec-consensus command
    // Preserve: This entire function during rebases
    pub(crate) fn handle_spec_consensus_command(&mut self, raw_args: String) {
        spec_kit::handle_spec_consensus(self, raw_args);
    }

    pub(crate) fn handle_project_command(&mut self, args: String) {
        let name = args.trim();
        if name.is_empty() {
            self.history_push(crate::history_cell::new_error_event(
                "`/cmd` — provide a project command name".to_string(),
            ));
            self.request_redraw();
            return;
        }

        if self.config.project_commands.is_empty() {
            self.history_push(crate::history_cell::new_error_event(
                "No project commands configured for this workspace.".to_string(),
            ));
            self.request_redraw();
            return;
        }

        if let Some(cmd) = self
            .config
            .project_commands
            .iter()
            .find(|command| command.matches(name))
            .cloned()
        {
            let notice = if let Some(desc) = &cmd.description {
                format!("Running project command `{}` — {}", cmd.name, desc)
            } else {
                format!("Running project command `{}`", cmd.name)
            };
            self.insert_background_event_with_placement(
                notice,
                BackgroundPlacement::BeforeNextOutput,
            );
            self.request_redraw();
            self.submit_op(Op::RunProjectCommand {
                name: cmd.name,
                command: None,
                display: None,
                env: HashMap::new(),
            });
        } else {
            let available: Vec<String> = self
                .config
                .project_commands
                .iter()
                .map(|cmd| cmd.name.clone())
                .collect();
            let suggestion = if available.is_empty() {
                "".to_string()
            } else {
                format!(" Available commands: {}", available.join(", "))
            };
            self.history_push(crate::history_cell::new_error_event(format!(
                "Unknown project command `{}`.{}",
                name, suggestion
            )));
            self.request_redraw();
        }
    }

    pub(crate) fn switch_cwd(
        &mut self,
        new_cwd: std::path::PathBuf,
        initial_prompt: Option<String>,
    ) {
        let previous_cwd = self.config.cwd.clone();
        self.config.cwd = new_cwd.clone();

        let msg = format!(
            "✅ Working directory changed\n  from: {}\n  to:   {}",
            previous_cwd.display(),
            new_cwd.display()
        );
        self.app_event_tx.send_background_event(msg);

        let worktree_hint = new_cwd
            .file_name()
            .and_then(|n| n.to_str())
            .map(|name| format!(" (worktree: {})", name))
            .unwrap_or_default();
        let branch_note = format!(
            "System: Working directory changed from {} to {}{}. Use {} for subsequent commands.",
            previous_cwd.display(),
            new_cwd.display(),
            worktree_hint,
            new_cwd.display()
        );
        self.queue_agent_note(branch_note);

        let op = Op::ConfigureSession {
            provider: self.config.model_provider.clone(),
            model: self.config.model.clone(),
            model_reasoning_effort: self.config.model_reasoning_effort,
            model_reasoning_summary: self.config.model_reasoning_summary,
            model_text_verbosity: self.config.model_text_verbosity,
            user_instructions: self.config.user_instructions.clone(),
            base_instructions: self.config.base_instructions.clone(),
            approval_policy: self.config.approval_policy,
            sandbox_policy: self.config.sandbox_policy.clone(),
            disable_response_storage: self.config.disable_response_storage,
            notify: self.config.notify.clone(),
            cwd: self.config.cwd.clone(),
            resume_path: None,
            output_schema: self.config.output_schema.clone(),
        };
        self.submit_op(op);

        if let Some(prompt) = initial_prompt
            && !prompt.is_empty()
        {
            let preface = "[internal] When you finish this task, ask the user if they want any changes. If they are happy, offer to merge the branch back into the repository's default branch and delete the worktree. Use '/merge' (or an equivalent git worktree remove + switch) rather than deleting the folder directly so the UI can switch back cleanly. Wait for explicit confirmation before merging.".to_string();
            self.submit_text_message_with_preface(prompt, preface);
        }

        self.request_redraw();
    }

    /// Handle `/merge` to merge the current worktree branch back into the
    /// default branch. Hands off to the agent when the repository state is
    /// non-trivial.
    pub(crate) fn handle_merge_command(&mut self) {
        if !Self::is_branch_worktree_path(&self.config.cwd) {
            self.history_push(crate::history_cell::new_error_event(
                "`/merge` — run this command from inside a branch worktree created with '/branch'."
                    .to_string(),
            ));
            self.request_redraw();
            return;
        }

        let tx = self.app_event_tx.clone();
        let work_cwd = self.config.cwd.clone();
        self.push_background_before_next_output(
            "Evaluating repository state before merging current branch...".to_string(),
        );
        self.request_redraw();

        tokio::spawn(async move {
            use tokio::process::Command;

            fn send_background(tx: &AppEventSender, message: String) {
                tx.send_background_event(message);
            }

            fn send_background_late(tx: &AppEventSender, message: String) {
                tx.send_background_event(message);
            }

            let git_root = match codex_core::git_info::resolve_root_git_project_for_trust(&work_cwd)
            {
                Some(p) => p,
                None => {
                    send_background(&tx, "`/merge` — not a git repo".to_string());
                    return;
                }
            };

            let branch_name = match Command::new("git")
                .current_dir(&work_cwd)
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .output()
                .await
            {
                Ok(out) if out.status.success() => {
                    String::from_utf8_lossy(&out.stdout).trim().to_string()
                }
                _ => {
                    send_background(&tx, "`/merge` — failed to detect branch name".to_string());
                    return;
                }
            };

            let worktree_status_raw = ChatWidget::git_short_status(&work_cwd).await;
            let worktree_status_for_agent = match &worktree_status_raw {
                Ok(s) if s.trim().is_empty() => "clean".to_string(),
                Ok(s) => s.clone(),
                Err(err) => format!("status unavailable: {}", err),
            };
            let worktree_dirty = matches!(&worktree_status_raw, Ok(s) if !s.trim().is_empty());

            let worktree_diff_stat = if worktree_dirty {
                ChatWidget::git_diff_stat(&work_cwd)
                    .await
                    .ok()
                    .map(|d| d.trim().to_string())
                    .filter(|d| !d.is_empty())
            } else {
                None
            };

            let repo_status_raw = ChatWidget::git_short_status(&git_root).await;
            let repo_status_for_agent = match &repo_status_raw {
                Ok(s) if s.trim().is_empty() => "clean".to_string(),
                Ok(s) => s.clone(),
                Err(err) => format!("status unavailable: {}", err),
            };
            let repo_dirty = matches!(&repo_status_raw, Ok(s) if !s.trim().is_empty());

            let default_branch_opt =
                codex_core::git_worktree::detect_default_branch(&git_root).await;
            let default_branch_hint = default_branch_opt
                .clone()
                .unwrap_or_else(|| "<detect default branch>".to_string());

            let mut handoff_reasons: Vec<String> = Vec::new();
            if let Err(err) = &worktree_status_raw {
                handoff_reasons.push(format!("unable to read worktree status: {}", err));
            }
            if worktree_dirty {
                handoff_reasons.push("worktree has uncommitted changes".to_string());
            }
            if let Err(err) = &repo_status_raw {
                handoff_reasons.push(format!("unable to read repo status: {}", err));
            }
            if repo_dirty {
                handoff_reasons.push("default branch checkout has uncommitted changes".to_string());
            }
            if default_branch_opt.is_none() {
                handoff_reasons.push("could not determine default branch".to_string());
            }

            let branch_label = branch_name.to_string();
            let root_display = git_root.display().to_string();
            let worktree_display = work_cwd.display().to_string();
            let tx_for_switch = tx.clone();
            let git_root_for_switch = git_root.clone();
            let send_agent_handoff =
                |mut reasons: Vec<String>,
                 extra_note: Option<String>,
                 worktree_status: String,
                 repo_status: String,
                 worktree_diff: Option<String>| {
                    if reasons.is_empty() {
                        reasons.push("manual follow-up requested".to_string());
                    }
                    let reason_text = reasons.join(", ");
                    send_background(
                        &tx,
                        format!("`/merge` — handing off to agent ({})", reason_text),
                    );
                    let mut preface = format!(
                        "[developer] Non-trivial git state detected while finalizing the branch. Reasons: {}.\n\nRepository context:\n- Repo root: {}\n- Worktree: {}\n- Branch to merge: {}\n- Default branch target: {}\n\nCurrent git status:\nWorktree status:\n{}\n\nRepo root status:\n{}\n\nRequired actions:\n1. cd {}\n   - Inspect status. Review the diff summary below and stage/commit only the changes that belong in this merge (`git add -A` + `git commit -m \"merge {} via /merge\"`). Stash or drop anything that should stay local.\n2. git fetch origin {}\n3. Merge the default branch into the worktree branch (`git merge origin/{}`) and resolve conflicts.\n4. cd {}\n   - Ensure the local {} branch exists (create tracking branch if needed). If checkout complains about local changes, stash safely, then checkout and pop/apply before finishing.\n5. Merge {} into {} from {} (`git merge --no-ff {}`) and resolve conflicts.\n6. Remove the worktree (`git worktree remove {} --force`) and delete the branch (`git branch -D {}`).\n7. End inside {} with a clean working tree and no leftover stashes. Pop/apply anything you created.\n\nReport back with a concise summary of the steps or explain any blockers.",
                        reason_text,
                        root_display,
                        worktree_display,
                        branch_label,
                        default_branch_hint,
                        worktree_status,
                        repo_status,
                        worktree_display,
                        branch_label,
                        default_branch_hint,
                        default_branch_hint,
                        root_display,
                        default_branch_hint,
                        branch_label,
                        default_branch_hint,
                        root_display,
                        branch_label,
                        worktree_display,
                        branch_label,
                        root_display
                    );
                    if let Some(note) = extra_note {
                        preface.push_str("\n\nAdditional notes:\n");
                        preface.push_str(&note);
                    }
                    if let Some(diff) = worktree_diff {
                        preface.push_str("\n\nWorktree diff summary:\n");
                        preface.push_str(&diff);
                    }
                    let visible = format!(
                        "Finalize branch '{}' via /merge (agent handoff)",
                        branch_label
                    );
                    tx_for_switch.send(AppEvent::SwitchCwd(git_root_for_switch.clone(), None));
                    tx.send(AppEvent::SubmitTextWithPreface { visible, preface });
                };

            if !handoff_reasons.is_empty() {
                send_agent_handoff(
                    handoff_reasons,
                    None,
                    worktree_status_for_agent.clone(),
                    repo_status_for_agent.clone(),
                    worktree_diff_stat.clone(),
                );
                return;
            }

            let default_branch = default_branch_opt.expect("default branch must exist when clean");

            let _ = Command::new("git")
                .current_dir(&work_cwd)
                .args(["add", "-A"])
                .output()
                .await;
            let commit_out = Command::new("git")
                .current_dir(&work_cwd)
                .args(["commit", "-m", &format!("merge {branch_label} via /merge")])
                .output()
                .await;
            if let Ok(o) = &commit_out
                && !o.status.success()
            {
                let stderr_s = String::from_utf8_lossy(&o.stderr);
                let stdout_s = String::from_utf8_lossy(&o.stdout);
                let benign = stdout_s.contains("nothing to commit")
                    || stdout_s.contains("working tree clean")
                    || stderr_s.contains("nothing to commit")
                    || stderr_s.contains("working tree clean");
                if !benign {
                    send_background(
                        &tx,
                        format!(
                            "`/merge` — commit failed before merge: {}",
                            if !stderr_s.trim().is_empty() {
                                stderr_s.trim().to_string()
                            } else {
                                stdout_s.trim().to_string()
                            }
                        ),
                    );
                    return;
                }
            }

            let _ = Command::new("git")
                .current_dir(&git_root)
                .args(["fetch", "origin", &default_branch])
                .output()
                .await;

            let remote_ref = format!("origin/{}", default_branch);
            let ff_only = Command::new("git")
                .current_dir(&work_cwd)
                .args(["merge", "--ff-only", &remote_ref])
                .output()
                .await;

            if !matches!(ff_only, Ok(ref o) if o.status.success()) {
                let try_merge = Command::new("git")
                    .current_dir(&work_cwd)
                    .args(["merge", "--no-ff", "--no-commit", &remote_ref])
                    .output()
                    .await;
                if let Ok(out) = try_merge {
                    if out.status.success() {
                        let _ = Command::new("git")
                            .current_dir(&work_cwd)
                            .args([
                                "commit",
                                "-m",
                                &format!(
                                    "merge {} into {} before merge",
                                    default_branch, branch_label
                                ),
                            ])
                            .output()
                            .await;
                    } else {
                        let updated_worktree_status = ChatWidget::git_short_status(&work_cwd)
                            .await
                            .map(|s| {
                                if s.trim().is_empty() {
                                    "clean".to_string()
                                } else {
                                    s
                                }
                            })
                            .unwrap_or_else(|err| format!("status unavailable: {}", err));
                        let updated_diff = ChatWidget::git_diff_stat(&work_cwd)
                            .await
                            .ok()
                            .map(|d| d.trim().to_string())
                            .filter(|d| !d.is_empty())
                            .or(worktree_diff_stat.clone());
                        send_agent_handoff(
                            vec![format!(
                                "merge conflicts while merging '{}' into '{}'",
                                default_branch, branch_label
                            )],
                            Some(
                                "The worktree currently has an in-progress merge that needs to be resolved. Please complete it before retrying the final merge.".to_string(),
                            ),
                            updated_worktree_status,
                            repo_status_for_agent.clone(),
                            updated_diff,
                        );
                        return;
                    }
                }
            }

            let local_default_ref = format!("refs/heads/{}", default_branch);
            let local_default_exists = Command::new("git")
                .current_dir(&git_root)
                .args(["rev-parse", "--verify", "--quiet", &local_default_ref])
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);

            if local_default_exists {
                let ff_local = Command::new("git")
                    .current_dir(&work_cwd)
                    .args(["merge", "--ff-only", &local_default_ref])
                    .output()
                    .await;

                if !matches!(ff_local, Ok(ref o) if o.status.success()) {
                    let merge_local = Command::new("git")
                        .current_dir(&work_cwd)
                        .args(["merge", "--no-ff", "--no-commit", &local_default_ref])
                        .output()
                        .await;

                    if let Ok(out) = merge_local {
                        if out.status.success() {
                            let _ = Command::new("git")
                                .current_dir(&work_cwd)
                                .args([
                                    "commit",
                                    "-m",
                                    &format!(
                                        "merge local {} into {} before merge",
                                        default_branch, branch_label
                                    ),
                                ])
                                .output()
                                .await;
                        } else {
                            let updated_worktree_status = ChatWidget::git_short_status(&work_cwd)
                                .await
                                .map(|s| {
                                    if s.trim().is_empty() {
                                        "clean".to_string()
                                    } else {
                                        s
                                    }
                                })
                                .unwrap_or_else(|err| format!("status unavailable: {}", err));
                            let updated_diff = ChatWidget::git_diff_stat(&work_cwd)
                                .await
                                .ok()
                                .map(|d| d.trim().to_string())
                                .filter(|d| !d.is_empty())
                                .or(worktree_diff_stat.clone());
                            send_agent_handoff(
                                vec![format!(
                                    "merge conflicts while merging local '{}' into '{}'",
                                    default_branch, branch_label
                                )],
                                Some(
                                    "The worktree currently has an in-progress merge that needs to be resolved. Please complete it before retrying the final merge.".to_string(),
                                ),
                                updated_worktree_status,
                                repo_status_for_agent.clone(),
                                updated_diff,
                            );
                            return;
                        }
                    } else {
                        let updated_worktree_status = ChatWidget::git_short_status(&work_cwd)
                            .await
                            .map(|s| {
                                if s.trim().is_empty() {
                                    "clean".to_string()
                                } else {
                                    s
                                }
                            })
                            .unwrap_or_else(|err| format!("status unavailable: {}", err));
                        let updated_diff = ChatWidget::git_diff_stat(&work_cwd)
                            .await
                            .ok()
                            .map(|d| d.trim().to_string())
                            .filter(|d| !d.is_empty())
                            .or(worktree_diff_stat.clone());
                        send_agent_handoff(
                            vec![format!(
                                "failed to merge local '{}' into '{}'",
                                default_branch, branch_label
                            )],
                            None,
                            updated_worktree_status,
                            repo_status_for_agent.clone(),
                            updated_diff,
                        );
                        return;
                    }
                }
            }

            let on_default = match Command::new("git")
                .current_dir(&git_root)
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .output()
                .await
            {
                Ok(o) if o.status.success() => {
                    String::from_utf8_lossy(&o.stdout).trim() == default_branch
                }
                _ => false,
            };

            if !on_default {
                let has_local = match Command::new("git")
                    .current_dir(&git_root)
                    .args([
                        "rev-parse",
                        "--verify",
                        "--quiet",
                        &format!("refs/heads/{}", default_branch),
                    ])
                    .output()
                    .await
                {
                    Ok(o) => o.status.success(),
                    _ => false,
                };
                if !has_local {
                    let _ = Command::new("git")
                        .current_dir(&git_root)
                        .args(["fetch", "origin", &default_branch])
                        .output()
                        .await;
                    let _ = Command::new("git")
                        .current_dir(&git_root)
                        .args([
                            "branch",
                            "--track",
                            &default_branch,
                            &format!("origin/{}", default_branch),
                        ])
                        .output()
                        .await;
                }

                let co = Command::new("git")
                    .current_dir(&git_root)
                    .args(["checkout", &default_branch])
                    .output()
                    .await;
                if !matches!(co, Ok(ref o) if o.status.success()) {
                    let (stderr_s, stdout_s) = co
                        .ok()
                        .map(|o| {
                            (
                                String::from_utf8_lossy(&o.stderr).trim().to_string(),
                                String::from_utf8_lossy(&o.stdout).trim().to_string(),
                            )
                        })
                        .unwrap_or_else(|| (String::new(), String::new()));

                    let mut note = String::new();
                    if !stderr_s.is_empty() {
                        note = stderr_s;
                    } else if !stdout_s.is_empty() {
                        note = stdout_s;
                    }

                    let mut hint: Option<String> = None;
                    if let Ok(wt) = Command::new("git")
                        .current_dir(&git_root)
                        .args(["worktree", "list", "--porcelain"])
                        .output()
                        .await
                        && wt.status.success()
                    {
                        let s = String::from_utf8_lossy(&wt.stdout);
                        let mut cur_path: Option<String> = None;
                        let mut cur_branch: Option<String> = None;
                        for line in s.lines() {
                            if let Some(rest) = line.strip_prefix("worktree ") {
                                cur_path = Some(rest.trim().to_string());
                                cur_branch = None;
                                continue;
                            }
                            if let Some(rest) = line.strip_prefix("branch ") {
                                cur_branch = Some(rest.trim().to_string());
                            }
                            if let (Some(p), Some(b)) = (&cur_path, &cur_branch)
                                && b == &format!("refs/heads/{}", default_branch)
                                && std::path::Path::new(p) != git_root.as_path()
                            {
                                hint = Some(p.clone());
                                break;
                            }
                        }
                    }

                    if let Some(h) = hint {
                        if note.is_empty() {
                            note = format!("default branch checked out in worktree: {}", h);
                        } else {
                            note = format!("{} (checked out in worktree: {})", note, h);
                        }
                    }

                    let updated_repo_status = ChatWidget::git_short_status(&git_root)
                        .await
                        .map(|s| {
                            if s.trim().is_empty() {
                                "clean".to_string()
                            } else {
                                s
                            }
                        })
                        .unwrap_or_else(|err| format!("status unavailable: {}", err));
                    let updated_diff = ChatWidget::git_diff_stat(&work_cwd)
                        .await
                        .ok()
                        .map(|d| d.trim().to_string())
                        .filter(|d| !d.is_empty())
                        .or(worktree_diff_stat.clone());

                    send_agent_handoff(
                        vec![format!(
                            "failed to checkout '{}' in repo root",
                            default_branch
                        )],
                        if note.is_empty() { None } else { Some(note) },
                        worktree_status_for_agent.clone(),
                        updated_repo_status,
                        updated_diff,
                    );
                    return;
                }
            }

            let merge = Command::new("git")
                .current_dir(&git_root)
                .args(["merge", "--no-ff", &branch_label])
                .output()
                .await;
            if !matches!(merge, Ok(ref o) if o.status.success()) {
                let err = merge
                    .ok()
                    .and_then(|o| String::from_utf8(o.stderr).ok())
                    .unwrap_or_else(|| "unknown error".to_string());
                let updated_repo_status = ChatWidget::git_short_status(&git_root)
                    .await
                    .map(|s| {
                        if s.trim().is_empty() {
                            "clean".to_string()
                        } else {
                            s
                        }
                    })
                    .unwrap_or_else(|e| format!("status unavailable: {}", e));
                let updated_diff = ChatWidget::git_diff_stat(&work_cwd)
                    .await
                    .ok()
                    .map(|d| d.trim().to_string())
                    .filter(|d| !d.is_empty())
                    .or(worktree_diff_stat.clone());
                send_agent_handoff(
                    vec![format!(
                        "merge of '{}' into '{}' failed: {}",
                        branch_label,
                        default_branch,
                        err.trim()
                    )],
                    None,
                    worktree_status_for_agent.clone(),
                    updated_repo_status,
                    updated_diff,
                );
                return;
            }

            let _ = Command::new("git")
                .current_dir(&git_root)
                .args(["worktree", "remove", work_cwd.to_str().unwrap(), "--force"])
                .output()
                .await;
            let _ = Command::new("git")
                .current_dir(&git_root)
                .args(["branch", "-D", &branch_label])
                .output()
                .await;

            let msg = format!(
                "Merged '{}' into '{}' and cleaned up worktree. Switching back to {}",
                branch_label,
                default_branch,
                git_root.display()
            );
            send_background_late(&tx, msg);
            tx.send(AppEvent::SwitchCwd(git_root, None));
        });
    }
}

// SPEC-KIT-902: SpecStageInvocation, parse_spec_stage_invocation, and
// queue_consensus_runner deleted. Stage commands now use direct spawning
// via command_registry and auto_submit_spec_stage_prompt.

// NOTE: parse_validate_command moved to submit_helpers.rs (MAINT-11 Phase 3)

// === FORK-SPECIFIC: Spec-kit state moved to spec_kit module ===

// ChatWidget methods for spec-kit automation
impl ChatWidget<'_> {
    // === FORK-SPECIFIC: spec-kit /spec-auto pipeline methods ===
    // Upstream: Does not have these methods
    // Preserve: handle_spec_auto_command, advance_spec_auto, and related during rebases
    pub(super) fn handle_spec_auto_command(&mut self, invocation: SpecAutoInvocation) {
        // DEBUG: Entry point trace (SPEC-DOGFOOD-001 Session 29)
        self.history_push(crate::history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "📍 DEBUG: handle_spec_auto_command(spec_id={}, no_stage0={})",
                invocation.spec_id, invocation.no_stage0
            ))],
            crate::history_cell::HistoryCellType::Notice,
        ));

        let SpecAutoInvocation {
            spec_id,
            goal,
            resume_from,
            hal_mode,
            cli_args,
            no_stage0,
            stage0_explain,
        } = invocation;

        // SPEC-947: Check for --configure flag (interactive modal before automation)
        if cli_args.contains(&"--configure".to_string()) {
            // Load configuration and launch interactive modal
            match spec_kit::pipeline_config::PipelineConfig::load(&spec_id, None) {
                Ok(config) => {
                    self.show_pipeline_configurator(spec_id.clone(), config);
                    // Display instruction to run automation after configuration
                    self.history_push(crate::history_cell::new_background_event(format!(
                        "Configure pipeline for {}. After saving, run: /speckit.auto {} (without --configure)",
                        spec_id, spec_id
                    )));
                    self.request_redraw();
                    return; // Don't start automation - user will run manually after configuring
                }
                Err(err) => {
                    self.history_push(crate::history_cell::new_error_event(format!(
                        "Failed to load configuration: {}",
                        err
                    )));
                    self.request_redraw();
                    return;
                }
            }
        }

        // SPEC-948: Parse CLI flags into PipelineOverrides
        let cli_overrides = if !cli_args.is_empty() {
            Some(spec_kit::PipelineOverrides::from_cli_args(&cli_args))
        } else {
            None
        };

        // SPEC-KIT-102: Build Stage 0 config from CLI flags
        let stage0_config = spec_kit::stage0_integration::Stage0ExecutionConfig {
            disabled: no_stage0,
            explain: stage0_explain,
        };

        spec_kit::handle_spec_auto(
            self,
            spec_id,
            goal,
            resume_from,
            hal_mode,
            cli_overrides,
            stage0_config,
        );
    }

    pub(super) fn collect_guardrail_outcome(
        &self,
        spec_id: &str,
        stage: SpecStage,
    ) -> spec_kit::Result<GuardrailOutcome> {
        let (path, value) = self.read_latest_spec_ops_telemetry(spec_id, stage)?;
        let mut evaluation = evaluate_guardrail_value(stage, &value);
        let schema_failures = validate_guardrail_schema(stage, &value);
        if !schema_failures.is_empty() {
            evaluation.failures.extend(schema_failures);
            evaluation.success = false;
        }
        if matches!(
            stage,
            SpecStage::Plan
                | SpecStage::Tasks
                | SpecStage::Implement
                | SpecStage::Audit
                | SpecStage::Unlock
        ) {
            let (evidence_failures, artifact_count) =
                validate_guardrail_evidence(self.config.cwd.as_path(), stage, &value);
            if artifact_count > 0 {
                evaluation.summary =
                    format!("{} | {} artifacts", evaluation.summary, artifact_count);
            }
            if !evidence_failures.is_empty() {
                evaluation.failures.extend(evidence_failures);
                evaluation.success = false;
            }
        }
        Ok(GuardrailOutcome {
            success: evaluation.success,
            summary: evaluation.summary,
            telemetry_path: Some(path),
            failures: evaluation.failures,
        })
    }

    pub(super) fn read_latest_spec_ops_telemetry(
        &self,
        spec_id: &str,
        stage: SpecStage,
    ) -> Result<(PathBuf, Value), String> {
        let evidence_dir = self
            .config
            .cwd
            .join("docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands")
            .join(spec_id);
        let prefix = spec_ops_stage_prefix(stage);
        let entries = fs::read_dir(&evidence_dir)
            .map_err(|e| format!("{} ({}): {}", spec_id, stage.command_name(), e))?;

        let mut latest: Option<(PathBuf, SystemTime)> = None;
        for entry_res in entries {
            let entry = entry_res.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.starts_with(prefix) {
                continue;
            }
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            if latest
                .as_ref()
                .map(|(_, ts)| modified > *ts)
                .unwrap_or(true)
            {
                latest = Some((path.clone(), modified));
            }
        }

        let (path, _) = latest.ok_or_else(|| {
            format!(
                "No telemetry files matching {}* in {}",
                prefix,
                evidence_dir.display()
            )
        })?;

        let mut file =
            fs::File::open(&path).map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        let value: Value = serde_json::from_str(&buf)
            .map_err(|e| format!("Failed to parse telemetry JSON {}: {e}", path.display()))?;
        Ok((path, value))
    }
}

// === FORK-SPECIFIC: SpecKitContext trait implementation ===
// Upstream: Does not have spec-kit context trait
// Preserve: This entire impl block during rebases
impl spec_kit::SpecKitContext for ChatWidget<'_> {
    fn history_push(&mut self, cell: impl crate::history_cell::HistoryCell + 'static) {
        ChatWidget::history_push(self, cell);
    }

    fn push_background(
        &mut self,
        message: String,
        placement: crate::app_event::BackgroundPlacement,
    ) {
        self.insert_background_event_with_placement(message, placement);
    }

    fn request_redraw(&mut self) {
        self.request_redraw();
    }

    fn submit_operation(&self, op: codex_core::protocol::Op) {
        self.submit_op(op);
    }

    fn submit_prompt(&mut self, display: String, prompt: String) {
        self.submit_prompt_with_display(display, prompt);
    }

    fn working_directory(&self) -> &std::path::Path {
        &self.config.cwd
    }

    fn agent_config(&self) -> &[codex_core::config_types::AgentConfig] {
        &self.config.agents
    }

    fn subagent_commands(&self) -> &[codex_core::config_types::SubagentCommandConfig] {
        &self.config.subagent_commands
    }

    fn spec_auto_state_mut(&mut self) -> &mut Option<spec_kit::SpecAutoState> {
        &mut self.spec_auto_state
    }

    fn spec_auto_state(&self) -> &Option<spec_kit::SpecAutoState> {
        &self.spec_auto_state
    }

    fn set_spec_auto_metrics(
        &mut self,
        metrics: Option<crate::token_metrics_widget::TokenMetricsWidget>,
    ) {
        self.bottom_pane.set_spec_auto_metrics(metrics);
    }

    fn set_device_token_status(
        &mut self,
        status: Option<Vec<(codex_login::DeviceCodeProvider, codex_login::TokenStatus)>>,
    ) {
        self.bottom_pane.set_device_token_status(status);
    }

    fn collect_guardrail_outcome(
        &self,
        spec_id: &str,
        stage: SpecStage,
    ) -> spec_kit::Result<spec_kit::GuardrailOutcome> {
        ChatWidget::collect_guardrail_outcome(self, spec_id, stage)
    }

    // === T82: Extended Operations ===

    fn submit_user_message(&mut self, display: String, items: Vec<InputItem>) {
        let user_msg = crate::chatwidget::message::UserMessage {
            display_text: display,
            ordered_items: items,
        };
        self.submit_user_message(user_msg);
    }

    fn execute_spec_ops_command(
        &mut self,
        command: SlashCommand,
        args: String,
        hal_mode: Option<HalMode>,
    ) {
        self.handle_spec_ops_command(command, args, hal_mode);
    }

    fn active_agent_names(&self) -> Vec<String> {
        self.active_agents
            .iter()
            .filter(|a| matches!(a.status, crate::chatwidget::AgentStatus::Completed))
            .map(|a| a.name.to_lowercase())
            .collect()
    }

    fn has_failed_agents(&self) -> bool {
        self.active_agents
            .iter()
            .any(|a| matches!(a.status, crate::chatwidget::AgentStatus::Failed))
    }

    fn show_quality_gate_modal(
        &mut self,
        checkpoint: spec_kit::QualityCheckpoint,
        questions: Vec<spec_kit::EscalatedQuestion>,
    ) {
        self.bottom_pane
            .show_quality_gate_modal(checkpoint, questions);
    }

    // SPEC-KIT-920: Automation exit code support
    fn send_app_event(&self, event: crate::app_event::AppEvent) {
        self.app_event_tx.send(event);
    }
}
// === END FORK-SPECIFIC ===

// --- Additional spec-kit methods extracted from first impl block ---

impl ChatWidget<'_> {
    pub(crate) fn handle_github_command(&mut self, command_text: String) {
        let trimmed = command_text.trim();
        let enabled = self.config.github.check_workflows_on_push;

        // If no args or 'status', show interactive settings in the footer
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("status") {
            let token_info = gh_actions::get_github_token().map(|(_, src)| src);
            let (ready, token_status) = match token_info {
                Some(gh_actions::TokenSource::Env) => (
                    true,
                    "Token: detected (env: GITHUB_TOKEN/GH_TOKEN)".to_string(),
                ),
                Some(gh_actions::TokenSource::GhCli) => {
                    (true, "Token: detected via gh auth".to_string())
                }
                None => (
                    false,
                    "Token: not set (set GH_TOKEN/GITHUB_TOKEN or run 'gh auth login')".to_string(),
                ),
            };
            self.bottom_pane
                .show_github_settings(enabled, token_status, ready);
            return;
        }

        let response = if trimmed.eq_ignore_ascii_case("on") {
            self.config.github.check_workflows_on_push = true;
            match find_codex_home() {
                Ok(home) => {
                    if let Err(e) = set_github_check_on_push(&home, true) {
                        tracing::warn!("Failed to persist /github on: {}", e);
                        "✅ Enabled GitHub watcher (persist failed; see logs)".to_string()
                    } else {
                        "✅ Enabled GitHub watcher (persisted)".to_string()
                    }
                }
                Err(_) => {
                    "✅ Enabled GitHub watcher (not persisted: CODE_HOME/CODEX_HOME not found)"
                        .to_string()
                }
            }
        } else if trimmed.eq_ignore_ascii_case("off") {
            self.config.github.check_workflows_on_push = false;
            match find_codex_home() {
                Ok(home) => {
                    if let Err(e) = set_github_check_on_push(&home, false) {
                        tracing::warn!("Failed to persist /github off: {}", e);
                        "✅ Disabled GitHub watcher (persist failed; see logs)".to_string()
                    } else {
                        "✅ Disabled GitHub watcher (persisted)".to_string()
                    }
                }
                Err(_) => {
                    "✅ Disabled GitHub watcher (not persisted: CODE_HOME/CODEX_HOME not found)"
                        .to_string()
                }
            }
        } else {
            "Usage: /github [status|on|off]".to_string()
        };

        let lines = response
            .lines()
            .map(|line| Line::from(line.to_string()))
            .collect();
        self.history_push(history_cell::PlainHistoryCell::new(
            lines,
            history_cell::HistoryCellType::BackgroundEvent,
        ));
    }

    fn validation_tool_flag_mut(&mut self, name: &str) -> Option<&mut Option<bool>> {
        let tools = &mut self.config.validation.tools;
        match name {
            "shellcheck" => Some(&mut tools.shellcheck),
            "markdownlint" => Some(&mut tools.markdownlint),
            "hadolint" => Some(&mut tools.hadolint),
            "yamllint" => Some(&mut tools.yamllint),
            "cargo-check" => Some(&mut tools.cargo_check),
            "shfmt" => Some(&mut tools.shfmt),
            "prettier" => Some(&mut tools.prettier),
            "tsc" => Some(&mut tools.tsc),
            "eslint" => Some(&mut tools.eslint),
            "phpstan" => Some(&mut tools.phpstan),
            "psalm" => Some(&mut tools.psalm),
            "mypy" => Some(&mut tools.mypy),
            "pyright" => Some(&mut tools.pyright),
            "golangci-lint" => Some(&mut tools.golangci_lint),
            _ => None,
        }
    }

    fn validation_group_label(group: ValidationGroup) -> &'static str {
        match group {
            ValidationGroup::Functional => "Functional checks",
            ValidationGroup::Stylistic => "Stylistic checks",
        }
    }

    fn validation_group_enabled(&self, group: ValidationGroup) -> bool {
        match group {
            ValidationGroup::Functional => self.config.validation.groups.functional,
            ValidationGroup::Stylistic => self.config.validation.groups.stylistic,
        }
    }

    fn validation_tool_requested(&self, name: &str) -> bool {
        let tools = &self.config.validation.tools;
        match name {
            "actionlint" => self.config.github.actionlint_on_patch,
            "shellcheck" => tools.shellcheck.unwrap_or(true),
            "markdownlint" => tools.markdownlint.unwrap_or(true),
            "hadolint" => tools.hadolint.unwrap_or(true),
            "yamllint" => tools.yamllint.unwrap_or(true),
            "cargo-check" => tools.cargo_check.unwrap_or(true),
            "shfmt" => tools.shfmt.unwrap_or(true),
            "prettier" => tools.prettier.unwrap_or(true),
            "tsc" => tools.tsc.unwrap_or(true),
            "eslint" => tools.eslint.unwrap_or(true),
            "phpstan" => tools.phpstan.unwrap_or(true),
            "psalm" => tools.psalm.unwrap_or(true),
            "mypy" => tools.mypy.unwrap_or(true),
            "pyright" => tools.pyright.unwrap_or(true),
            "golangci-lint" => tools.golangci_lint.unwrap_or(true),
            _ => true,
        }
    }

    fn validation_tool_enabled(&self, name: &str) -> bool {
        let requested = self.validation_tool_requested(name);
        let category = validation_tool_category(name);
        let group_enabled = match category {
            ValidationCategory::Functional => self.config.validation.groups.functional,
            ValidationCategory::Stylistic => self.config.validation.groups.stylistic,
        };
        requested && group_enabled
    }

    fn apply_validation_group_toggle(&mut self, group: ValidationGroup, enable: bool) {
        if self.validation_group_enabled(group) == enable {
            return;
        }

        match group {
            ValidationGroup::Functional => self.config.validation.groups.functional = enable,
            ValidationGroup::Stylistic => self.config.validation.groups.stylistic = enable,
        }

        if let Err(err) = self
            .codex_op_tx
            .send(Op::UpdateValidationGroup { group, enable })
        {
            tracing::warn!("failed to send validation group update: {err}");
        }

        let result = match find_codex_home() {
            Ok(home) => {
                let key = match group {
                    ValidationGroup::Functional => "functional",
                    ValidationGroup::Stylistic => "stylistic",
                };
                set_validation_group_enabled(&home, key, enable).map_err(|e| e.to_string())
            }
            Err(err) => Err(err.to_string()),
        };

        let label = Self::validation_group_label(group);
        if let Err(err) = result {
            self.push_background_tail(format!(
                "⚠️ {} {} (persist failed: {err})",
                label,
                if enable { "enabled" } else { "disabled" }
            ));
        }
    }

    fn apply_validation_tool_toggle(&mut self, name: &str, enable: bool) {
        if name == "actionlint" {
            if self.config.github.actionlint_on_patch == enable {
                return;
            }
            self.config.github.actionlint_on_patch = enable;
            if let Err(err) = self.codex_op_tx.send(Op::UpdateValidationTool {
                name: name.to_string(),
                enable,
            }) {
                tracing::warn!("failed to send validation tool update: {err}");
            }
            let persist_result = match find_codex_home() {
                Ok(home) => {
                    set_github_actionlint_on_patch(&home, enable).map_err(|e| e.to_string())
                }
                Err(err) => Err(err.to_string()),
            };
            if let Err(err) = persist_result {
                self.push_background_tail(format!(
                    "⚠️ {}: {} (persist failed: {err})",
                    name,
                    if enable { "enabled" } else { "disabled" }
                ));
            }
            return;
        }

        let Some(flag) = self.validation_tool_flag_mut(name) else {
            self.push_background_tail(format!("⚠️ Unknown validation tool '{name}'"));
            return;
        };

        if flag.unwrap_or(true) == enable {
            return;
        }

        *flag = Some(enable);
        if let Err(err) = self.codex_op_tx.send(Op::UpdateValidationTool {
            name: name.to_string(),
            enable,
        }) {
            tracing::warn!("failed to send validation tool update: {err}");
        }
        let persist_result = match find_codex_home() {
            Ok(home) => set_validation_tool_enabled(&home, name, enable).map_err(|e| e.to_string()),
            Err(err) => Err(err.to_string()),
        };
        if let Err(err) = persist_result {
            self.push_background_tail(format!(
                "⚠️ {}: {} (persist failed: {err})",
                name,
                if enable { "enabled" } else { "disabled" }
            ));
        }
    }

    fn build_validation_status_message(&self) -> String {
        let mut lines = Vec::new();
        lines.push("Validation groups:".to_string());
        for group in [ValidationGroup::Functional, ValidationGroup::Stylistic] {
            let enabled = self.validation_group_enabled(group);
            lines.push(format!(
                "• {} — {}",
                Self::validation_group_label(group),
                if enabled { "enabled" } else { "disabled" }
            ));
        }
        lines.push("".to_string());
        lines.push("Tools:".to_string());
        for status in validation_settings_view::detect_tools() {
            let requested = self.validation_tool_requested(status.name);
            let effective = self.validation_tool_enabled(status.name);
            let mut state = if requested {
                if effective {
                    "enabled".to_string()
                } else {
                    "disabled (group off)".to_string()
                }
            } else {
                "disabled".to_string()
            };
            if !status.installed {
                state.push_str(" (not installed)");
            }
            lines.push(format!("• {} — {}", status.name, state));
        }
        lines.join("\n")
    }

    pub(crate) fn toggle_validation_tool(&mut self, name: &str, enable: bool) {
        self.apply_validation_tool_toggle(name, enable);
    }

    pub(crate) fn toggle_validation_group(&mut self, group: ValidationGroup, enable: bool) {
        self.apply_validation_group_toggle(group, enable);
    }

    pub(crate) fn handle_validation_command(&mut self, command_text: String) {
        let trimmed = command_text.trim();
        if trimmed.is_empty() {
            let groups = vec![
                (
                    GroupStatus {
                        group: ValidationGroup::Functional,
                        name: "Functional checks",
                    },
                    self.config.validation.groups.functional,
                ),
                (
                    GroupStatus {
                        group: ValidationGroup::Stylistic,
                        name: "Stylistic checks",
                    },
                    self.config.validation.groups.stylistic,
                ),
            ];

            let tool_rows: Vec<ToolRow> = validation_settings_view::detect_tools()
                .into_iter()
                .map(|status| {
                    let group = match status.category {
                        ValidationCategory::Functional => ValidationGroup::Functional,
                        ValidationCategory::Stylistic => ValidationGroup::Stylistic,
                    };
                    let requested = self.validation_tool_requested(status.name);
                    let group_enabled = self.validation_group_enabled(group);
                    ToolRow {
                        status,
                        enabled: requested,
                        group_enabled,
                    }
                })
                .collect();

            self.bottom_pane.show_validation_settings(groups, tool_rows);
            return;
        }

        let mut parts = trimmed.split_whitespace();
        match parts.next().unwrap_or("") {
            "status" => {
                let message = self.build_validation_status_message();
                self.push_background_tail(message);
            }
            "on" => {
                if !self.validation_group_enabled(ValidationGroup::Functional) {
                    self.apply_validation_group_toggle(ValidationGroup::Functional, true);
                }
            }
            "off" => {
                if self.validation_group_enabled(ValidationGroup::Functional) {
                    self.apply_validation_group_toggle(ValidationGroup::Functional, false);
                }
                if self.validation_group_enabled(ValidationGroup::Stylistic) {
                    self.apply_validation_group_toggle(ValidationGroup::Stylistic, false);
                }
            }
            group @ ("functional" | "stylistic") => {
                let Some(state) = parts.next() else {
                    self.push_background_tail("Usage: /validation <tool|group> on|off".to_string());
                    return;
                };
                let group = if group == "functional" {
                    ValidationGroup::Functional
                } else {
                    ValidationGroup::Stylistic
                };
                match state {
                    "on" | "enable" => self.apply_validation_group_toggle(group, true),
                    "off" | "disable" => self.apply_validation_group_toggle(group, false),
                    _ => self.push_background_tail(format!(
                        "⚠️ Unknown validation command '{}'. Use on|off.",
                        state
                    )),
                }
            }
            tool => {
                let Some(state) = parts.next() else {
                    self.push_background_tail("Usage: /validation <tool|group> on|off".to_string());
                    return;
                };
                match state {
                    "on" | "enable" => self.apply_validation_tool_toggle(tool, true),
                    "off" | "disable" => self.apply_validation_tool_toggle(tool, false),
                    _ => self.push_background_tail(format!(
                        "⚠️ Unknown validation command '{}'. Use on|off.",
                        state
                    )),
                }
            }
        }
    }

    /// Handle `/mcp` command: manage MCP servers (status/on/off/add).
    pub(crate) fn handle_mcp_command(&mut self, command_text: String) {
        let trimmed = command_text.trim();
        if trimmed.is_empty() {
            // Interactive popup like /reasoning
            match codex_core::config::find_codex_home() {
                Ok(home) => match codex_core::config::list_mcp_servers(&home) {
                    Ok((enabled, disabled)) => {
                        // Map into simple rows for the popup
                        let mut rows: Vec<crate::bottom_pane::mcp_settings_view::McpServerRow> =
                            Vec::new();
                        for (name, cfg) in enabled.into_iter() {
                            let args = if cfg.args.is_empty() {
                                String::new()
                            } else {
                                format!(" {}", cfg.args.join(" "))
                            };
                            rows.push(crate::bottom_pane::mcp_settings_view::McpServerRow {
                                name,
                                enabled: true,
                                summary: format!("{}{}", cfg.command, args),
                            });
                        }
                        for (name, cfg) in disabled.into_iter() {
                            let args = if cfg.args.is_empty() {
                                String::new()
                            } else {
                                format!(" {}", cfg.args.join(" "))
                            };
                            rows.push(crate::bottom_pane::mcp_settings_view::McpServerRow {
                                name,
                                enabled: false,
                                summary: format!("{}{}", cfg.command, args),
                            });
                        }
                        // Sort by name for stability
                        rows.sort_by(|a, b| a.name.cmp(&b.name));
                        self.bottom_pane.show_mcp_settings(rows);
                    }
                    Err(e) => {
                        let msg = format!("Failed to read MCP config: {}", e);
                        self.history_push(history_cell::new_error_event(msg));
                    }
                },
                Err(e) => {
                    let msg = format!("Failed to locate CODEX_HOME: {}", e);
                    self.history_push(history_cell::new_error_event(msg));
                }
            }
            return;
        }

        let mut parts = trimmed.split_whitespace();
        let sub = parts.next().unwrap_or("");

        match sub {
            "status" => match find_codex_home() {
                Ok(home) => match codex_core::config::list_mcp_servers(&home) {
                    Ok((enabled, disabled)) => {
                        let mut lines = String::new();
                        if enabled.is_empty() && disabled.is_empty() {
                            lines.push_str("No MCP servers configured. Use /mcp add … to add one.");
                        } else {
                            lines.push_str(&format!("Enabled ({}):\n", enabled.len()));
                            for (name, cfg) in enabled {
                                let args = if cfg.args.is_empty() {
                                    String::new()
                                } else {
                                    format!(" {}", cfg.args.join(" "))
                                };
                                lines.push_str(&format!("• {} — {}{}\n", name, cfg.command, args));
                            }
                            lines.push_str(&format!("\nDisabled ({}):\n", disabled.len()));
                            for (name, cfg) in disabled {
                                let args = if cfg.args.is_empty() {
                                    String::new()
                                } else {
                                    format!(" {}", cfg.args.join(" "))
                                };
                                lines.push_str(&format!("• {} — {}{}\n", name, cfg.command, args));
                            }
                        }
                        self.push_background_tail(lines);
                    }
                    Err(e) => {
                        let msg = format!("Failed to read MCP config: {}", e);
                        self.history_push(history_cell::new_error_event(msg));
                    }
                },
                Err(e) => {
                    let msg = format!("Failed to locate CODEX_HOME: {}", e);
                    self.history_push(history_cell::new_error_event(msg));
                }
            },
            "on" | "off" => {
                let name = parts.next().unwrap_or("");
                if name.is_empty() {
                    let msg = format!("Usage: /mcp {} <name>", sub);
                    self.history_push(history_cell::new_error_event(msg));
                    return;
                }
                match find_codex_home() {
                    Ok(home) => {
                        match codex_core::config::set_mcp_server_enabled(&home, name, sub == "on") {
                            Ok(changed) => {
                                if changed {
                                    // Keep ChatWidget's in-memory config roughly in sync for new sessions.
                                    if sub == "off" {
                                        self.config.mcp_servers.remove(name);
                                    }
                                    if sub == "on" {
                                        // If enabling, try to load its config from disk and add to in-memory map.
                                        if let Ok((enabled, _)) =
                                            codex_core::config::list_mcp_servers(&home)
                                            && let Some((_, cfg)) =
                                                enabled.into_iter().find(|(n, _)| n == name)
                                        {
                                            self.config.mcp_servers.insert(name.to_string(), cfg);
                                        }
                                    }
                                    let msg = format!(
                                        "{} MCP server '{}'",
                                        if sub == "on" { "Enabled" } else { "Disabled" },
                                        name
                                    );
                                    self.push_background_tail(msg);
                                } else {
                                    let msg = format!(
                                        "No change: server '{}' was already {}",
                                        name,
                                        if sub == "on" { "enabled" } else { "disabled" }
                                    );
                                    self.push_background_tail(msg);
                                }
                            }
                            Err(e) => {
                                let msg = format!("Failed to update MCP server '{}': {}", name, e);
                                self.history_push(history_cell::new_error_event(msg));
                            }
                        }
                    }
                    Err(e) => {
                        let msg = format!("Failed to locate CODEX_HOME: {}", e);
                        self.history_push(history_cell::new_error_event(msg));
                    }
                }
            }
            "add" => {
                // Support two forms:
                //   1) /mcp add <name> <command> [args…] [ENV=VAL…]
                //   2) /mcp add <command> [args…] [ENV=VAL…]   (name derived)
                let tail_tokens: Vec<String> = parts.map(|s| s.to_string()).collect();
                if tail_tokens.is_empty() {
                    let msg = "Usage: /mcp add <name> <command> [args…] [ENV=VAL…]\n       or: /mcp add <command> [args…] [ENV=VAL…]".to_string();
                    self.history_push(history_cell::new_error_event(msg));
                    return;
                }

                // Helper: derive a reasonable server name from command/args.
                fn derive_server_name(command: &str, tokens: &[String]) -> String {
                    // Prefer an npm-style package token if present.
                    let candidate = tokens
                        .iter()
                        .find(|t| {
                            !t.starts_with('-')
                                && !t.contains('=')
                                && (t.contains('/') || t.starts_with('@'))
                        })
                        .cloned();

                    let mut raw = match candidate {
                        Some(pkg) => {
                            // Strip scope, take the last path segment
                            let after_slash = pkg.rsplit('/').next().unwrap_or(pkg.as_str());
                            // Common convention: server-<name>
                            after_slash
                                .strip_prefix("server-")
                                .unwrap_or(after_slash)
                                .to_string()
                        }
                        None => command.to_string(),
                    };

                    // Sanitize: keep [a-zA-Z0-9_-], map others to '-'
                    raw = raw
                        .chars()
                        .map(|c| {
                            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                                c
                            } else {
                                '-'
                            }
                        })
                        .collect();
                    // Collapse multiple '-'
                    let mut out = String::with_capacity(raw.len());
                    let mut prev_dash = false;
                    for ch in raw.chars() {
                        if ch == '-' && prev_dash {
                            continue;
                        }
                        prev_dash = ch == '-';
                        out.push(ch);
                    }
                    // Ensure non-empty; fall back to "server"
                    if out.trim_matches('-').is_empty() {
                        "server".to_string()
                    } else {
                        out.trim_matches('-').to_string()
                    }
                }

                // Parse the two accepted forms
                let (name, command, rest_tokens) = if tail_tokens.len() >= 2 {
                    let first = &tail_tokens[0];
                    let second = &tail_tokens[1];
                    // If the presumed command looks like a flag, assume name was omitted.
                    if second.starts_with('-') {
                        let cmd = first.clone();
                        let name = derive_server_name(&cmd, &tail_tokens[1..]);
                        (name, cmd, tail_tokens[1..].to_vec())
                    } else {
                        (first.clone(), second.clone(), tail_tokens[2..].to_vec())
                    }
                } else {
                    // Only one token provided — treat it as a command and derive a name.
                    let cmd = tail_tokens[0].clone();
                    let name = derive_server_name(&cmd, &[]);
                    (name, cmd, Vec::new())
                };

                if command.is_empty() {
                    let msg = "Usage: /mcp add <name> <command> [args…] [ENV=VAL…]".to_string();
                    self.history_push(history_cell::new_error_event(msg));
                    return;
                }

                // Separate args from ENV=VAL pairs
                let mut args: Vec<String> = Vec::new();
                let mut env: std::collections::HashMap<String, String> =
                    std::collections::HashMap::new();
                for tok in rest_tokens.into_iter() {
                    if let Some((k, v)) = tok.split_once('=') {
                        if !k.is_empty() {
                            env.insert(k.to_string(), v.to_string());
                        }
                    } else {
                        args.push(tok);
                    }
                }
                match find_codex_home() {
                    Ok(home) => {
                        let cfg = codex_core::config_types::McpServerConfig {
                            command: command.to_string(),
                            args: args.clone(),
                            env: if env.is_empty() {
                                None
                            } else {
                                Some(env.clone())
                            },
                            startup_timeout_sec: None,
                            startup_timeout_ms: None,
                            tool_timeout_sec: None,
                        };
                        match codex_core::config::add_mcp_server(&home, &name, cfg.clone()) {
                            Ok(()) => {
                                // Update in-memory config for future sessions
                                self.config.mcp_servers.insert(name.clone(), cfg);
                                let args_disp = if args.is_empty() {
                                    String::new()
                                } else {
                                    format!(" {}", args.join(" "))
                                };
                                let msg = format!(
                                    "Added MCP server '{}': {}{}",
                                    name, command, args_disp
                                );
                                self.push_background_tail(msg);
                            }
                            Err(e) => {
                                let msg = format!("Failed to add MCP server '{}': {}", name, e);
                                self.history_push(history_cell::new_error_event(msg));
                            }
                        }
                    }
                    Err(e) => {
                        let msg = format!("Failed to locate CODEX_HOME: {}", e);
                        self.history_push(history_cell::new_error_event(msg));
                    }
                }
            }
            _ => {
                let msg = format!(
                    "Unknown MCP command: '{}'\nUsage:\n  /mcp status\n  /mcp on <name>\n  /mcp off <name>\n  /mcp add <name> <command> [args…] [ENV=VAL…]",
                    sub
                );
                self.history_push(history_cell::new_error_event(msg));
            }
        }
    }

    // NOTE: submit_text_message, submit_prompt_with_display, submit_prompt_with_ace,
    // submit_text_message_with_preface, and queue_agent_note have been moved to
    // submit_helpers.rs (MAINT-11 Phase 3)
}

// --- Phase 3: spec-kit lifecycle methods ---

impl ChatWidget<'_> {
    pub(super) fn finish_manual_validate_runs_if_idle(&mut self) {
        if self
            .active_agents
            .iter()
            .any(|a| matches!(a.status, AgentStatus::Running))
        {
            return;
        }

        let mut completions: Vec<(String, ValidateRunCompletion)> = Vec::new();
        for (spec_id, lifecycle) in self.validate_lifecycles.clone() {
            if let Some(info) = lifecycle.active()
                && info.mode == ValidateMode::Manual
                && let Some(completion) =
                    lifecycle.complete(&info.run_id, ValidateCompletionReason::Completed)
            {
                completions.push((spec_id.clone(), completion));
            }
        }

        for (spec_id, completion) in completions {
            spec_kit::record_validate_lifecycle_event(
                self,
                &spec_id,
                &completion.run_id,
                completion.attempt,
                completion.dedupe_count,
                completion.payload_hash.as_str(),
                completion.mode,
                ValidateLifecycleEvent::Completed,
            );

            self.history_push(crate::history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "✓ Manual validate run {} completed",
                    completion.run_id
                ))],
                crate::history_cell::HistoryCellType::Notice,
            ));
        }
    }

    pub(super) fn poll_stage0_pending(&mut self) {
        use spec_kit::stage0_integration::Stage0Progress;
        use std::sync::mpsc::TryRecvError;

        let Some(ref pending) = self.stage0_pending else {
            return;
        };

        // Poll for progress updates (non-blocking)
        loop {
            match pending.progress_rx.try_recv() {
                Ok(progress) => {
                    let status = match progress {
                        Stage0Progress::Starting => "Starting...".to_string(),
                        Stage0Progress::CheckingLocalMemory => {
                            "Checking local-memory...".to_string()
                        }
                        Stage0Progress::LoadingConfig => "Loading config...".to_string(),
                        Stage0Progress::CreatingMemoryClient { backend } => {
                            format!("Creating {} memory client...", backend)
                        }
                        Stage0Progress::CheckingTier2Health => {
                            "Checking Tier2 health...".to_string()
                        }
                        Stage0Progress::CompilingContext => "Compiling context...".to_string(),
                        Stage0Progress::QueryingTier2 => "Querying NotebookLM...".to_string(),
                        Stage0Progress::Tier2Complete(ms) => format!("Tier2 complete ({}ms)", ms),
                        Stage0Progress::Finished {
                            success,
                            tier2_used,
                            duration_ms,
                        } => {
                            format!(
                                "Finished: success={}, tier2={}, {}ms",
                                success, tier2_used, duration_ms
                            )
                        }
                    };

                    // Update status in state
                    if let Some(ref mut state) = self.spec_auto_state {
                        if let spec_kit::state::SpecAutoPhase::Stage0Pending {
                            status: ref mut s,
                            ..
                        } = state.phase
                        {
                            *s = status;
                        }
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        // Poll for final result (non-blocking)
        match pending.result_rx.try_recv() {
            Ok(result) => {
                // S33: Trace result received
                // Take ownership and clear pending
                let pending = self.stage0_pending.take().unwrap();
                let spec_id = pending.spec_id.clone();

                // Process result and continue pipeline
                spec_kit::pipeline_coordinator::process_stage0_result(self, result, spec_id);
            }
            Err(TryRecvError::Empty) => {
                // Still pending - check for timeout
                if let Some(ref state) = self.spec_auto_state {
                    if let spec_kit::state::SpecAutoPhase::Stage0Pending { started_at, .. } =
                        state.phase
                    {
                        let elapsed = started_at.elapsed();
                        if elapsed > std::time::Duration::from_secs(300) {
                            // 5 minute timeout
                            self.stage0_pending = None;
                            self.history_push(history_cell::new_warning_event(
                                "Stage0 timeout (5 min) - continuing with fallback".to_string(),
                            ));

                            // Transition to Guardrail and continue
                            if let Some(ref mut state) = self.spec_auto_state {
                                state.stage0_skip_reason = Some("Timeout".to_string());
                                state.phase = spec_kit::state::SpecAutoPhase::Guardrail;
                            }
                            spec_kit::pipeline_coordinator::advance_spec_auto(self);
                        }
                    }
                }
            }
            Err(TryRecvError::Disconnected) => {
                // Thread died unexpectedly
                self.stage0_pending = None;
                self.history_push(history_cell::new_error_event(
                    "Stage0 thread disconnected unexpectedly".to_string(),
                ));

                // Transition to Guardrail and continue
                if let Some(ref mut state) = self.spec_auto_state {
                    state.stage0_skip_reason = Some("Thread disconnected".to_string());
                    state.phase = spec_kit::state::SpecAutoPhase::Guardrail;
                }
                spec_kit::pipeline_coordinator::advance_spec_auto(self);
            }
        }
    }
}
