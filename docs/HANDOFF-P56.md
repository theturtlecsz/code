# P56 Handoff - `/speckit.project` Command Implementation

**Generated**: 2025-11-29
**Previous Session**: P55 (SPEC-KIT-900 Reanalysis → E2E Strategy Pivot)
**Base Commit**: TBD (commit P55 changes first)

---

## Session Goal

Implement `/speckit.project` command - the missing piece for true end-to-end spec-kit workflow.

**Scope**: SPEC creation + implementation (defer E2E test to P57)

---

## Context: Why This Command

### P55 Discovery

SPEC-KIT-900 was designed to validate spec-kit by running spec-kit on itself (circular). P55 analysis revealed:

1. **Real E2E test** = Create fresh project → run pipeline → validate output compiles/works
2. **Gap identified**: No command to scaffold a new project from scratch
3. **Current state**: `/speckit.new` assumes project exists, creates SPEC only

### The Missing Workflow Step

```
CURRENT:
  (project already exists) → /speckit.new → /speckit.auto

NEEDED:
  /speckit.project → (creates scaffold) → /speckit.new → /speckit.auto
```

---

## User Decisions (P55)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language support | **Research needed** | Analyze current structure; consider agnostic + language-specific templates |
| Auto-create SPEC | **No, separate** | /speckit.project scaffolds only; user runs /speckit.new after |
| Default location | **Current directory** | Create in ./project-name from wherever command runs |
| P56 Scope | **SPEC + implement** | Defer E2E test (miniserve clone) to P57 |

---

## Implementation Tasks

### Task 1: Research & Analysis (30-45 min)

**Objective**: Determine optimal template strategy

**Questions to answer**:
1. What does current project structure assume? (Cargo workspace? Single crate?)
2. How do existing /speckit.* commands detect project type?
3. What files does CLAUDE.md reference that need to exist?
4. What language-specific templates would be valuable?
   - Rust: Cargo.toml, src/main.rs or src/lib.rs, tests/
   - Python: pyproject.toml, src/, tests/, __init__.py
   - TypeScript: package.json, tsconfig.json, src/, tests/
   - Generic: git, docs/, CLAUDE.md only

**Research locations**:
```
codex-rs/tui/src/slash_commands/       # Existing command implementations
codex-rs/tui/src/spec_kit/             # Spec-kit infrastructure
~/code/CLAUDE.md                       # Project context expectations
```

### Task 2: Create SPEC (30 min)

**Location**: `docs/SPEC-KIT-9XX-project-command/spec.md`

**SPEC structure**:
- Problem statement (the gap)
- Functional requirements
- CLI interface design
- Template structure (per language)
- Success criteria
- Non-goals

### Task 3: Implementation (1-2 hours)

**New files**:
```
codex-rs/tui/src/slash_commands/speckit_project.rs   # Command handler
codex-rs/tui/src/spec_kit/templates/                 # Template files
  ├── rust/
  │   ├── Cargo.toml.template
  │   ├── main.rs.template
  │   └── gitignore.template
  ├── python/
  │   └── ...
  └── generic/
      ├── CLAUDE.md.template
      └── gitignore.template
```

**Integration points**:
- Register in slash_commands/mod.rs
- Add to command autocomplete
- Wire up in app.rs routing

### Task 4: Validation (30 min)

**Test the command**:
```bash
cd /tmp
/speckit.project miniserve-clone --lang rust

# Verify structure:
ls -la miniserve-clone/
cat miniserve-clone/Cargo.toml
cat miniserve-clone/CLAUDE.md
git -C miniserve-clone log --oneline
```

---

## CLI Interface Design (Draft)

```
/speckit.project <name> [options]

Arguments:
  <name>              Project name (creates ./name/ directory)

Options:
  --lang <LANG>       Language template: rust, python, typescript, generic
                      Default: rust (or auto-detect from context?)
  --path <PATH>       Override location (default: current directory)
  --description <D>   Project description for README/CLAUDE.md
  --no-git            Skip git initialization
  --with-spec         Also run /speckit.new after scaffolding

Examples:
  /speckit.project myapp
  /speckit.project myapp --lang python --description "A CLI tool for X"
  /speckit.project myapp --path ~/projects/
```

---

## E2E Test Plan (Deferred to P57)

**Objective**: Replicate miniserve (OSS HTTP file server) using spec-kit

**Workflow**:
```bash
# P57 Session
cd ~
/speckit.project miniserve-clone --lang rust --description "Simple HTTP file server"
cd miniserve-clone
/speckit.new "HTTP file server with directory listing, file upload, basic auth, custom port"
/speckit.auto SPEC-001
cargo build && cargo test
./target/debug/miniserve-clone --help

# Compare to original
diff -r src/ ~/reference/miniserve/src/  # Structure comparison
tokei .                                   # LOC comparison
```

**Success criteria**:
- [ ] Project compiles
- [ ] Basic functionality works (serve files)
- [ ] Tests pass (if generated)
- [ ] Comparable structure to original

---

## Files Modified in P55

```
SPEC.md                    # Updated SPEC-KIT-900 status (line 77, 181)
docs/HANDOFF-P55.md        # Previous handoff (read-only reference)
docs/HANDOFF-P56.md        # This file
```

---

## Session Start Prompt

```
I'm continuing from P55. Goal: Implement /speckit.project command.

Context:
- P55 identified gap: no command to scaffold new projects
- User decisions: separate from /speckit.new, current dir default, research language templates
- Scope: SPEC + implementation (E2E test deferred to P57)

Tasks:
1. Research current project structure assumptions (30 min)
2. Create SPEC for /speckit.project command (30 min)
3. Implement the command with templates (1-2 hours)
4. Validate with test project (30 min)

Start by analyzing:
- codex-rs/tui/src/slash_commands/ (existing patterns)
- codex-rs/tui/src/spec_kit/ (infrastructure)
- CLAUDE.md (project context expectations)

Then create SPEC-KIT-9XX for the new command.
```

---

## Reference: miniserve Features (for P57)

From https://github.com/svenstaro/miniserve (~3k LOC):

**Core features**:
- Serve directory over HTTP
- Directory listing (file sizes, dates, icons)
- File upload support
- Basic authentication
- Custom port/bind address
- HTTPS with self-signed cert
- QR code for easy mobile access
- Compression (gzip)
- Range requests (resume downloads)
- CORS headers

**For SPEC prompt (simplified)**:
```
HTTP file server CLI in Rust with:
- Serve any directory on specified port
- Directory listing with file metadata
- Single file upload via POST
- Optional basic auth (--username/--password)
- Configurable bind address and port
- Clean shutdown on Ctrl+C
```

---

## P55 Session Summary

### Completed
1. Loaded and analyzed HANDOFF-P55.md
2. Read SPEC-KIT-900 PRD and current test infrastructure
3. Identified circularity problem (testing spec-kit with spec-kit)
4. Pivoted to true E2E strategy: fresh project + OSS comparison
5. Selected miniserve as comparison target
6. Identified /speckit.project gap
7. Updated SPEC.md with accurate SPEC-KIT-900 status
8. Gathered user decisions via questions

### Key Insight
SPEC-KIT-900 should become a **guided manual walkthrough**, not automated self-test. Real E2E validation = generate working software in fresh project.

### Decisions Captured
- /speckit.project: separate from /speckit.new
- Default: current directory
- Language: research agnostic + specific templates
- P56: implement command
- P57: run E2E test with miniserve
