# SPEC-945 Implementation Series - Session Start Prompt

**Last Updated**: 2025-11-14
**Current Phase**: SPEC-945D Phase 1 Planning Complete → Implementation Starting
**Repository**: https://github.com/theturtlecsz/code (FORK of just-every/code)

---

## Quick Context

We are working through the **SPEC-945 Implementation Series** - 6 detailed implementation specs created from research phase (SPEC-KIT-945). These specs provide production-ready implementation guidance for database integrity, async orchestration, retry logic, configuration management, benchmarking, and policy compliance.

**Working Directory**: `/home/thetu/code/codex-rs`
**Current Branch**: `feature/spec-945d-config-hot-reload`
**Progress**: 1/6 specs complete (SPEC-945C ✅), SPEC-945D in progress

---

## SPEC-945 Series Status

| SPEC    | Title                          | Status       | Priority | Est. Time | PR/Branch                                  | Completion Date |
|---------|--------------------------------|--------------|----------|-----------|--------------------------------------------|-----------------|
| ✅ 945C | SQLite Retry Mechanism         | **COMPLETE** | HIGH     | 3 days    | PR #6 (merged to main)                     | 2025-11-14      |
| 🔄 945D | Config Hot Reload              | **ACTIVE**   | HIGH     | 32h       | feature/spec-945d-config-hot-reload        | -               |
| 📋 945A | Async Orchestration            | Queued       | MEDIUM   | 20-25h    | -                                          | -               |
| 📋 945B | SQLite Transactions            | Queued       | MEDIUM   | 18-22h    | -                                          | -               |
| 📋 945E | Benchmarking & Instrumentation | Queued       | LOW      | 12-16h    | -                                          | -               |
| 📋 945F | Policy Compliance & OAuth2     | Queued       | LOW      | 8-10h     | -                                          | -               |

**Total Progress**: 1/6 complete (~17%, 3 days of ~100 hours total)
**Current Sprint**: SPEC-945D Phase 1.1 (Layered Config Loading)

---

## SPEC-945C Completion Summary

**PR #6**: https://github.com/theturtlecsz/code/pull/6 (merged 2025-11-14)
**Commit**: ad9c47e15 "feat(spec-945c): Implement SQLite retry mechanism with exponential backoff"

### What Was Delivered

**Implementation** (+449 LOC core code):
- Synchronous retry wrapper: `execute_with_backoff_sync()` (spec-kit/src/retry/strategy.rs)
- Error classification for rusqlite, DbError, SpecKitError
- 12 operations wrapped with retry logic (11 consensus_db + 1 evidence)

**Tests** (41/41 passing, 100%):
- spec-kit: 25/25 tests ✅
- read_path_migration: 8/8 tests ✅
- write_path_cutover: 8/8 tests ✅

**Documentation** (36 pages):
- implementation.md (12 pages)
- developer-guide.md (8 pages)
- code-review-checklist.md (8 pages)
- validation-report.md (8 pages)

**Performance**:
- Happy path: <10µs overhead
- Single retry: ~100-150ms (100ms backoff + jitter)

**Pattern** (reusable for SPEC-945D):
```rust
execute_with_backoff_sync(
    &config,  // RetryConfig { max_attempts, initial_backoff, multiplier }
    || {
        // Critical operation (lock per attempt, not held across retries)
    }
)
```

---

## SPEC-945D Implementation Plan

**Goal**: Enable dynamic retry configuration updates without application restart

**Location**: `/home/thetu/code/docs/SPEC-945D-config-hot-reload/IMPLEMENTATION-PLAN.md`

### Implementation Phases

#### **Week 1: Core Configuration (16 hours)**

**Phase 1.1: Layered Config Loading (6h)** ← **CURRENT**
- Add dependencies (config-rs, toml, dirs)
- Create config module structure (mod.rs, loader.rs, error.rs)
- Implement ConfigLoader (defaults → file → env vars)
- Define config structs (AppConfig, ModelConfig, etc.)
- Unit tests (10-15 tests)

**Phase 1.2: JSON Schema Validation (5h)**
- Create config.schema.json (JSON Schema Draft 7)
- Implement SchemaValidator
- Integrate with ConfigLoader
- IDE integration (VS Code)

**Phase 1.3: Canonical Name Registry (5h)**
- Implement ModelRegistry (canonical name resolution)
- Error handling (unknown models, invalid counts)
- Quality gate validation

#### **Week 2: Hot-Reload & Integration (16 hours)**

**Phase 2.1: Filesystem Watching (6h)**
- Implement HotReloadWatcher with debouncing
- 2-second debounce window
- Atomic config replacement (Arc<RwLock>)

**Phase 2.2: TUI Integration (4h)**
- Integrate watcher with TUI event loop
- Idle vs busy state handling
- UI notifications (FileChanged, ReloadSuccess, ReloadFailed)

**Phase 2.3: Migration & Testing (6h)**
- Migrate hardcoded names to canonical names
- Backward compatibility
- End-to-end integration tests

---

## Session Verification Commands

Run these to verify current state before starting work:

```bash
# Navigate to working directory
cd /home/thetu/code/codex-rs

# Verify branch
git branch --show-current
# Expected: feature/spec-945d-config-hot-reload

# Verify base commit
git log --oneline -1 main
# Expected: ad9c47e15 feat(spec-945c): Implement SQLite retry mechanism...

# Check for uncommitted changes
git status --short
# Expected: clean (or only untracked session files)

# Verify tests still pass
cargo test -p codex-spec-kit retry 2>&1 | grep "test result"
# Expected: test result: ok. 18 passed; 0 failed...
```

---

## Retrieve Context from Local-Memory

Query for relevant previous work and patterns:

```bash
# Context query (use mcp__local-memory__search tool)
{
  "query": "SPEC-945 config hot reload implementation",
  "search_type": "semantic",
  "limit": 5,
  "tags": ["spec:SPEC-945D", "infrastructure"],
  "use_ai": true
}

# Recent architecture decisions
{
  "query": "configuration management layered config retry",
  "search_type": "semantic",
  "limit": 3,
  "domain": "infrastructure"
}
```

**Key Memory IDs**:
- f85a6573-0dae-4a4d-9ba0-4d439cc86b94: SPEC-945C completion (importance: 9)
- d7836d61-3628-4464-8b9a-b7cc470c8d50: SPEC-945C Day 3 milestone (importance: 9)

---

## Phase 1.1: Layered Config Loading (CURRENT TASK)

### Overview

**Goal**: Implement layered configuration with config-rs crate (12-factor app pattern)

**Estimated Time**: 6 hours

**Priority**: HIGH (blocks Phase 1.2 and all subsequent phases)

### Tasks

1. **Add Dependencies** (15 min)
   ```toml
   # Add to spec-kit/Cargo.toml [dependencies]
   config = "0.14"
   toml = "0.8"
   dirs = "5.0"
   ```

2. **Create Module Structure** (15 min)
   ```bash
   mkdir -p spec-kit/src/config
   touch spec-kit/src/config/mod.rs
   touch spec-kit/src/config/loader.rs
   touch spec-kit/src/config/error.rs
   ```

3. **Implement ConfigError** (30 min)
   ```rust
   // spec-kit/src/config/error.rs
   #[derive(Debug, thiserror::Error)]
   pub enum ConfigError {
       #[error("Config file error: {0}")]
       ConfigCrate(#[from] config::ConfigError),

       #[error("IO error: {0}")]
       Io(#[from] std::io::Error),

       #[error("Validation error: {0}")]
       Validation(String),
   }
   ```

4. **Define Config Structs** (1 hour)
   ```rust
   // spec-kit/src/config/loader.rs
   #[derive(Debug, Deserialize, Clone)]
   pub struct AppConfig {
       pub models: HashMap<String, ModelConfig>,
       pub quality_gates: QualityGateConfig,
       pub hot_reload: HotReloadConfig,
       pub validation: ValidationConfig,
   }

   #[derive(Debug, Deserialize, Clone)]
   pub struct ModelConfig {
       pub provider: String,  // "openai", "anthropic", "google"
       pub model: String,     // "gpt-4o", "claude-sonnet-4-5"
       pub tier: String,      // "low", "medium", "high"
   }

   // ... other structs
   ```

5. **Implement ConfigLoader** (2 hours)
   ```rust
   pub struct ConfigLoader {
       config_path: PathBuf,
   }

   impl ConfigLoader {
       pub fn new(config_path: PathBuf) -> Self {
           Self { config_path }
       }

       pub fn load(&self) -> Result<AppConfig, ConfigError> {
           let config = Config::builder()
               // Layer 1: Defaults
               .set_default("hot_reload.enabled", true)?
               .set_default("hot_reload.debounce_ms", 2000)?
               // Layer 2: File
               .add_source(File::with_name(&self.config_path.to_string_lossy()))
               // Layer 3: Env vars (SPECKIT_*)
               .add_source(Environment::with_prefix("SPECKIT").separator("__"))
               .build()?;

           Ok(config.try_deserialize()?)
       }
   }
   ```

6. **Write Unit Tests** (2 hours)
   - Test defaults loading
   - Test file override
   - Test environment variable override
   - Test layered priority (env > file > defaults)
   - Test missing file handling
   - Test invalid TOML handling

7. **Update Module Exports** (15 min)
   ```rust
   // spec-kit/src/config/mod.rs
   pub mod error;
   pub mod loader;

   pub use error::ConfigError;
   pub use loader::{AppConfig, ConfigLoader, ModelConfig, QualityGateConfig};
   ```

   ```rust
   // spec-kit/src/lib.rs
   pub mod config;  // Add this line
   ```

### Validation

```bash
# Build
cargo build -p codex-spec-kit

# Run tests
cargo test -p codex-spec-kit config::loader

# Expected output:
# test result: ok. 10-15 passed; 0 failed
```

### Deliverables

- ✅ Dependencies added to Cargo.toml
- ✅ Config module structure created
- ✅ ConfigError implemented
- ✅ Config structs defined
- ✅ ConfigLoader implemented (layered merging)
- ✅ Unit tests passing (10-15 tests)

### Success Criteria

- Loads defaults (embedded in code)
- Merges config file (TOML format)
- Applies environment variable overrides
- Priority: Env vars > File > Defaults
- Helpful error messages on failures

---

## Reference Documents

### SPEC-945D Specification
```bash
cat ../docs/SPEC-KIT-945-implementation-research/SPEC-945D-config-hot-reload.md
```

### Implementation Plan
```bash
cat ../docs/SPEC-945D-config-hot-reload/IMPLEMENTATION-PLAN.md
```

### SPEC-945C Patterns (for reference)
```bash
cat ../docs/SPEC-945C-sqlite-retry-mechanism/implementation.md
cat ../docs/SPEC-945C-sqlite-retry-mechanism/developer-guide.md
```

### Code References
- `codex-rs/spec-kit/src/retry/strategy.rs` - Retry patterns (SPEC-945C)
- `codex-rs/spec-kit/src/error.rs` - Error handling patterns
- `codex-rs/spec-kit/src/lib.rs` - Module structure example

---

## Development Workflow

### Quality Gates

Before committing:
1. **Format**: `cargo fmt --all`
2. **Lint**: `cargo clippy --workspace --all-targets --all-features`
3. **Build**: `cargo build -p codex-spec-kit`
4. **Test**: `cargo test -p codex-spec-kit`

### Git Workflow

```bash
# After completing Phase 1.1
git add spec-kit/Cargo.toml spec-kit/src/config/
git commit -m "feat(spec-945d): Implement layered config loading (Phase 1.1)

- Add config-rs, toml, dirs dependencies
- Create config module structure
- Implement ConfigLoader with layered merging (defaults → file → env)
- Define config structs (AppConfig, ModelConfig, QualityGateConfig, etc.)
- Add 15 unit tests for config loading (100% pass rate)

Part of SPEC-945D Phase 1 (Week 1, 6h)
"
```

---

## Success Metrics (from SPEC-945D)

| Metric | Target | Baseline | Validation Method |
|--------|--------|----------|-------------------|
| Config load time | <10ms | N/A | Benchmark test |
| Test pass rate | 100% | N/A | CI pipeline |
| Code coverage | >80% | N/A | cargo-tarpaulin |

---

## Common Issues & Solutions

### Issue: Cargo workspace location
**Solution**: Always run from `codex-rs/` directory
```bash
cd /home/thetu/code/codex-rs
cargo test -p codex-spec-kit
```

### Issue: Config file not found
**Pattern**: Use dirs crate for home directory
```rust
let config_path = dirs::home_dir()
    .ok_or(ConfigError::Validation("No home directory".into()))?
    .join(".code/config.toml");
```

### Issue: Environment variable naming
**Pattern**: Use SPECKIT_ prefix with __ separator
```bash
export SPECKIT_HOT_RELOAD__ENABLED=false
export SPECKIT_HOT_RELOAD__DEBOUNCE_MS=5000
```

### Issue: TOML parsing errors
**Solution**: Provide helpful error context
```rust
.add_source(File::with_name(&path).required(false))  // Don't fail if missing
// OR
.add_source(File::with_name(&path).required(true))   // Fail if missing
```

---

## After Phase 1.1 Completion

### Next Steps

**Immediate**:
1. Mark Phase 1.1 as complete in tracking
2. Commit changes with descriptive message
3. Move to Phase 1.2: JSON Schema Validation

**Phase 1.2 Preview** (5 hours):
- Add jsonschema dependency
- Create config.schema.json (JSON Schema Draft 7)
- Implement SchemaValidator
- Integrate validation into ConfigLoader
- IDE integration (VS Code settings)

### Update Tracking

Store milestone in local-memory (if Phase 1.1 > 6h or significant):
```rust
mcp__local-memory__store_memory:
- content: "SPEC-945D Phase 1.1 complete: Layered config loading implemented with config-rs. ConfigLoader supports defaults → file → env var merging. 15 unit tests passing. Pattern: Use config::Config::builder() with layered sources. Files: config/loader.rs, config/error.rs."
- domain: "infrastructure"
- tags: ["type:milestone", "spec:SPEC-945D", "config-management"]
- importance: 8
```

---

## 🎯 NEXT ITEM TO WORK ON

**START HERE**: Phase 1.1 - Layered Config Loading

**Immediate Actions**:
1. Add dependencies to `spec-kit/Cargo.toml` (config, toml, dirs)
2. Create config module structure (mkdir, touch files)
3. Implement ConfigError enum (error.rs)
4. Define config structs (AppConfig, ModelConfig, etc.)
5. Implement ConfigLoader with layered merging
6. Write unit tests (10-15 tests)
7. Validate: `cargo test -p codex-spec-kit config::loader`

**Estimated Time**: 6 hours
**Branch**: feature/spec-945d-config-hot-reload
**Priority**: HIGH (blocking all subsequent phases)

**Expected Output**:
- Dependencies added ✅
- Config module created ✅
- ConfigLoader working ✅
- 10-15 tests passing ✅
- Ready for Phase 1.2 (JSON Schema)

---

**Repository**: https://github.com/theturtlecsz/code
**Session Type**: Implementation (SPEC-945D Phase 1.1)
**Estimated Duration**: 6 hours
**Priority**: HIGH

**End of Session Prompt** - Ready to implement Phase 1.1!
