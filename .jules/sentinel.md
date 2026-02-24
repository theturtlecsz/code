## 2025-05-15 - Fragile Dangerous Command Detection
**Vulnerability:** The dangerous command detection logic for `rm` and `git` relied on fixed argument positions (e.g., checking `argv[1]`). This allowed bypasses like `rm -v -rf /` or `git --no-pager reset`.
**Learning:** Security heuristics based on command line arguments must account for flag reordering, combined flags (e.g., `-rf`), and global options appearing before subcommands.
**Prevention:** Use robust argument parsing or iterate over all arguments to detect dangerous flags/subcommands, rather than checking specific indices. When in doubt, fail safe (flag as dangerous).
