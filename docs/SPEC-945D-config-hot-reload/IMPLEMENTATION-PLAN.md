# SPEC-945D: Config Hot-Reload - Implementation Plan

**Created**: 2025-11-14
**Branch**: `feature/spec-945d-config-hot-reload`
**Estimated Time**: 32 hours (2 weeks)
**Dependencies**: SPEC-945C (SQLite Retry) ✅ Complete

---

## Overview

### Goal
Enable dynamic retry configuration updates without application restart through hot-reloadable configuration management.

### Key Features
1. **Layered Configuration** (12-factor app pattern): Defaults → File → Environment variables
2. **Hot-Reload**: Filesystem watching with debouncing (2-second window)
3. **JSON Schema Validation**: Fail-fast on invalid configs with helpful errors
4. **Canonical Model Names**: Single source of truth (eliminate `gemini` vs `gemini-flash` confusion)
5. **Configurable Quality Gates**: Per-stage agent selection from config file

### Success Criteria
- ✅ Config reload without restart (<100ms latency)
- ✅ Detection latency: 2-5 seconds (debounce window)
- ✅ CPU overhead: <1% (filesystem watcher)
- ✅ Validation prevents invalid states
- ✅ 100% test pass rate (following SPEC-945C pattern)

---

## Implementation Phases

### **Week 1: Core Configuration (16 hours)**

#### Phase 1.1: Layered Config Loading (Days 1-2, 6h)

**Tasks**:
1. Add dependencies to `spec-kit/Cargo.toml`:
   ```toml
   config = "0.14"
   toml = "0.8"
   dirs = "5.0"
   ```

2. Create config module structure:
   ```
   spec-kit/src/config/
   ├── mod.rs              # Module entry point
   ├── loader.rs           # ConfigLoader (layered loading)
   ├── models.rs           # ModelRegistry (canonical names)
   └── error.rs            # ConfigError types
   ```

3. Implement `ConfigLoader`:
   - Layered merging: Defaults → File → Environment variables
   - Support TOML format (`~/.code/config.toml`)
   - Environment variable overrides (`SPECKIT_*` prefix)

4. Define config structs:
   - `AppConfig` (top-level)
   - `ModelConfig` (provider, model, tier)
   - `QualityGateConfig` (per-stage agents)
   - `HotReloadConfig` (enabled, debounce_ms, watch_paths)
   - `ValidationConfig` (check_api_keys, check_commands, strict_schema)

5. Add unit tests:
   - Test defaults loading
   - Test file override
   - Test environment variable override
   - Test layered merging priority

**Deliverables**:
- `config/mod.rs`, `config/loader.rs`, `config/error.rs` implemented
- Unit tests passing (10-15 tests)
- Example `config.toml` created

**Validation**:
```bash
cargo test -p codex-spec-kit config::loader
```

---

#### Phase 1.2: JSON Schema Validation (Days 3-4, 5h)

**Tasks**:
1. Add dependency to `spec-kit/Cargo.toml`:
   ```toml
   jsonschema = "0.17"
   ```

2. Create `config.schema.json` (JSON Schema Draft 7):
   - Define `models` object (patternProperties for canonical names)
   - Define `quality_gates` object (plan, tasks, validate, audit, unlock)
   - Define `hot_reload` object (enabled, debounce_ms, watch_paths)
   - Define `validation` object (check_api_keys, check_commands, strict_schema)
   - Set required fields, additionalProperties: false

3. Implement `config/schema.rs`:
   - `SchemaValidator::from_file()` - Load schema from JSON
   - `SchemaValidator::validate()` - Validate config against schema
   - Detailed error messages with paths (`agents[0].canonical_name: missing`)

4. Integrate validation into `ConfigLoader::load()`:
   - Validate after deserialization
   - Fail fast with helpful errors
   - Allow disabling via `SPECKIT_VALIDATION__STRICT_SCHEMA=false`

5. Add schema validation tests:
   - Valid config passes
   - Invalid config fails with clear errors
   - Missing required fields detected
   - Unknown fields rejected

6. IDE integration (`.vscode/settings.json`):
   - JSON schema for TOML files (evenBetterToml extension)

**Deliverables**:
- `config.schema.json` (JSON Schema)
- `config/schema.rs` (SchemaValidator)
- `.vscode/settings.json` (IDE integration)
- Unit tests passing (8-10 tests)

**Validation**:
```bash
cargo test -p codex-spec-kit config::schema

# Manual validation (should fail)
echo 'invalid_field = "value"' >> ~/.code/config.toml
cargo run -p codex-tui  # Expect error: "Unknown field 'invalid_field'"
```

---

#### Phase 1.3: Canonical Name Registry (Day 5, 5h)

**Tasks**:
1. Implement `config/models.rs`:
   - `ModelRegistry::from_config()` - Build registry from config
   - `ModelRegistry::resolve()` - Canonical name → ModelConfig
   - `ModelRegistry::validate_quality_gate()` - Check agents exist
   - `ModelRegistry::list_available()` - List all models

2. Error handling:
   - `RegistryError::UnknownModel` with available options
   - `RegistryError::InvalidAgentCount` (1-5 agents per stage)

3. Add resolution tests:
   - Valid canonical name resolves
   - Invalid name returns error with suggestions
   - Quality gate validation (all agents exist)
   - Agent count validation (1-5 agents)

4. Update `spec-kit/src/lib.rs`:
   - Add `pub mod config;` to exports
   - Re-export `ConfigLoader`, `ModelRegistry`, `AppConfig`

**Deliverables**:
- `config/models.rs` (ModelRegistry)
- Unit tests passing (12-15 tests)
- Integration test (end-to-end name resolution)

**Validation**:
```bash
cargo test -p codex-spec-kit config::models
cargo test -p codex-spec-kit integration::config_resolution
```

---

### **Week 2: Hot-Reload & Integration (16 hours)**

#### Phase 2.1: Filesystem Watching (Days 1-2, 6h)

**Tasks**:
1. Add dependencies to `spec-kit/Cargo.toml`:
   ```toml
   notify = "6.1"
   notify-debouncer-full = "0.3"
   ```

2. Implement `config/hot_reload.rs`:
   - `HotReloadWatcher::new()` - Create watcher with debouncing
   - `HotReloadWatcher::get_config()` - Read current config (RwLock)
   - `HotReloadWatcher::recv_event()` - Receive reload events
   - `ConfigReloadEvent` enum (FileChanged, ReloadSuccess, ReloadFailed)

3. Debouncing logic:
   - 2-second debounce window (configurable)
   - Consolidate rapid file edits (WRITE + MODIFY + METADATA_CHANGE)
   - Single reload per debounce window

4. Atomic config replacement:
   - `Arc<RwLock<AppConfig>>` for concurrent access
   - Minimize write lock duration (<1ms)
   - Validate before replacing (rollback on validation failure)

5. Add debouncing tests:
   - Single reload on rapid edits (3 events → 1 reload)
   - Debounce window respected (2-second delay)
   - Failed reload preserves old config

**Deliverables**:
- `config/hot_reload.rs` (HotReloadWatcher)
- Unit tests passing (10-12 tests)
- Performance test (reload latency <100ms)

**Validation**:
```bash
cargo test -p codex-spec-kit config::hot_reload

# Manual test
cargo run -p codex-tui &
vim ~/.code/config.toml  # Edit file
# Expect: "Config file changed, reloading..."
# After 2s: "✅ Config reloaded successfully"
```

---

#### Phase 2.2: TUI Integration (Day 3, 4h)

**Tasks**:
1. Integrate watcher with TUI event loop:
   - Create `HotReloadWatcher` in TUI startup
   - Subscribe to reload events in `tokio::select!` loop
   - Handle `ConfigReloadEvent` variants

2. Idle vs busy state handling:
   - Check if quality gate is active before reload
   - Defer reload until quality gate completes
   - Show notification: "Config changed. Reload after completion?"

3. UI notifications:
   - "Config file changed, reloading..." (on FileChanged)
   - "✅ Config reloaded successfully" (on ReloadSuccess)
   - "❌ Config reload failed: <error>" (on ReloadFailed)

4. Component refresh on reload:
   - Refresh quality gate widget (new agents)
   - Refresh agent selection UI
   - Refresh validation layers

**Deliverables**:
- `tui/src/widgets/spec_kit/handler.rs` (reload event handling)
- Integration test (TUI reload without restart)

**Validation**:
```bash
# End-to-end test
cargo run -p codex-tui
# In another terminal:
vim ~/.code/config.toml  # Change plan agents
# TUI should show reload notification
```

---

#### Phase 2.3: Migration & Testing (Day 4, 6h)

**Tasks**:
1. Migrate hardcoded model names to canonical names:
   - Update `consensus.rs` - Use `registry.resolve(canonical_name)`
   - Update `quality_gates.rs` - Load agents from config (not hardcoded)
   - Remove hardcoded agent arrays

2. Backward compatibility:
   - Auto-migration from old `[[agents]]` array to new `[models]` map
   - Deprecation warnings in logs
   - Embedded defaults (no config file required)

3. Update test fixtures:
   - Replace hardcoded names with canonical names
   - Add test config files (valid, invalid, minimal)
   - Update integration tests

4. End-to-end testing:
   - Custom quality gate agents work
   - Hot-reload without restart works
   - Validation prevents invalid configs
   - Rollback on failed reload works

**Deliverables**:
- Migrated codebase (canonical names everywhere)
- Test fixtures updated
- Integration tests passing (20-25 tests)

**Validation**:
```bash
cargo test --workspace
cargo run -p codex-tui -- /speckit.plan SPEC-KIT-123
# Should use agents from config.toml, not hardcoded
```

---

## Documentation Plan (Week 3, 8-10 hours)

Following SPEC-945C pattern (36 pages total):

### 1. Implementation Guide (10-12 pages)
- Architecture overview
- Config schema reference
- Hot-reload cycle diagrams
- Performance characteristics
- Integration points

### 2. Developer Guide (6-8 pages)
- Quick start (creating config.toml)
- Canonical name resolution examples
- Hot-reload usage patterns
- Troubleshooting guide
- Common pitfalls

### 3. Code Review Checklist (6-8 pages)
- 35+ items covering:
  - Config validation
  - Hot-reload safety
  - Concurrency (RwLock usage)
  - Error handling
  - Test coverage

### 4. Validation Report (6-8 pages)
- Test results (100% pass rate)
- Performance benchmarks
- Integration validation
- Backward compatibility verification

---

## Risk Mitigation

### Risk 1: Hot-reload during quality gate transition
**Mitigation**:
- Lock quality gate state before reload check
- Defer reload if quality gate becomes active
- Comprehensive concurrency tests

### Risk 2: Invalid config breaks TUI startup
**Mitigation**:
- Validate on startup (fail fast)
- Use embedded defaults on validation failure (degraded mode)
- Log error with file path and line number

### Risk 3: JSON Schema too strict
**Mitigation**:
- `additionalProperties: false` only at top level
- Clear schema validation errors with hints
- Allow disabling via `SPECKIT_VALIDATION__STRICT_SCHEMA=false`

---

## Dependencies & Versions

### New Dependencies (to add to spec-kit/Cargo.toml)
```toml
[dependencies]
# Configuration
config = "0.14"               # Layered configuration
toml = "0.8"                  # TOML parsing
dirs = "5.0"                  # Home directory resolution

# Hot-reload
notify = "6.1"                # Filesystem watching
notify-debouncer-full = "0.3" # Debounced events

# Validation
jsonschema = "0.17"           # JSON Schema validation
```

### Existing Dependencies (already in Cargo.toml)
- ✅ `serde = "1.0"` (with derive)
- ✅ `serde_json = "1.0"`
- ✅ `thiserror = "2.0"`
- ✅ `anyhow = "1"`
- ✅ `tokio = "1"` (with sync, fs features)

---

## Success Metrics (from SPEC-945D)

| Metric | Target | Baseline | Validation Method |
|--------|--------|----------|-------------------|
| Reload latency (p95) | <100ms | N/A | Benchmark test |
| Detection latency | 2-5s | N/A | Debounce window test |
| CPU overhead | <1% | N/A | System profiling |
| Validation time | <50ms | N/A | Schema benchmark |
| Test pass rate | 100% | N/A | CI pipeline |

---

## Next Steps

**Immediate Actions** (start Week 1, Phase 1.1):
1. Add dependencies to `spec-kit/Cargo.toml`
2. Create `spec-kit/src/config/` directory structure
3. Implement `config/mod.rs` and `config/loader.rs`
4. Write first unit tests for config loading

**Branch**: `feature/spec-945d-config-hot-reload` ✅ Created
**Tracking**: Update SPEC.md with task status after each phase

---

**Expected Completion**: Week 2 Day 4 (32 hours total)
**Documentation**: Week 3 (8-10 hours)
**Total Estimate**: 40-42 hours (matches SPEC-945D estimate of 32h implementation + docs)
