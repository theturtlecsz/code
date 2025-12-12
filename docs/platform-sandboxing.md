### Platform sandboxing details

The mechanism used to implement the sandbox policy depends on your OS:

- **macOS 12+** uses **Apple Seatbelt** and runs commands using `sandbox-exec` with a profile (`-p`) that corresponds to the `--sandbox` that was specified.
- **Linux** uses a combination of Landlock/seccomp APIs to enforce the `sandbox` configuration.

Note that when running Linux in a containerized environment such as Docker, sandboxing may not work if the host/container configuration does not support the necessary Landlock/seccomp APIs. In such cases, configure your container so it provides the sandbox guarantees you need and then run `code` with `--sandbox danger-full-access` (or `--dangerously-bypass-approvals-and-sandbox`) within your container.
