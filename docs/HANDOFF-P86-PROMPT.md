# P86 Implementation Prompt – Extend Eval Harness for Code Lane & Code Hits

**Prior Session (P85) completed:**
- Code unit extraction via tree-sitter (CodeUnitExtractor in `tui/src/chatwidget/spec_kit/code_index.rs`)
- `/stage0.index` indexes both memories AND code units with `kind="code"`
- DCC code lane with `code_lane_enabled`, `code_top_k` config
- TASK_BRIEF "Code Context" section with Key Code Units + Other References
- All tests pass: 113 stage0 + 507 TUI

**Commit reference:** (to be added after commit)

---

## Goal

Extend the existing Stage 0 evaluation harness so you can quantitatively assess how well the code lane is working. P85 made code-unit indexing and code context real; P86 lets you ask: "For a spec that we *know* should surface `tui/src/...::handle_spec_auto`, does DCC actually put that in the top K code units?" and see P@K/R@K/MRR numbers.

---

## Design Decisions (Pre-Approved)

### 1. Include MRR alongside P@K and R@K
MRR (Mean Reciprocal Rank) tells you *where* in the top K the first relevant item appears. Critical for code lane where position matters.

### 2. Handle Missing Expected IDs: Hybrid Approach
- Warn and record missing expected IDs in `EvalResult.missing_expected_ids`
- Exclude missing IDs from metric denominators (don't count ingestion problems as ranking misses)
- Support `--strict` mode for CI that fails on any missing expected IDs

### 3. Built-in + External Eval Cases
- 2-3 hardcoded built-in cases for smoke testing/CI regression
- External JSON file support for growing, evolving eval corpus
- CLI shows source (builtin vs JSON) in output

### 4. Add /stage0.eval-code Shortcut
- New command `stage0.eval-code` as sugar for `--lane=code`
- Same underlying implementation as eval-backend
- Default k=10, can be overridden

---

## Implementation Tasks

### Task 1: Extend EvalCase for Lane-Specific Evaluation

**File:** `stage0/src/eval.rs` (create if needed)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EvalLane {
    Memory,
    Code,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCase {
    pub id: String,              // e.g. "code_stage0_handle_spec_auto"
    pub description: String,     // human-readable description
    pub lane: EvalLane,          // Memory or Code
    pub spec_id: String,         // SPEC ID to pass to Stage0
    pub spec_content: String,    // full or partial spec text
    pub expected_ids: Vec<String>, // expected memory IDs or code IDs
    #[serde(default)]
    pub k: Option<usize>,        // optional override for top-K
}
```

### Task 2: Extend EvalResult with MRR and Missing IDs

```rust
#[derive(Debug, Clone, Serialize)]
pub struct EvalResult {
    pub case_id: String,
    pub lane: EvalLane,
    pub k: usize,
    pub top_ids: Vec<String>,
    pub expected_ids: Vec<String>,
    pub hits: Vec<String>,
    pub missing_expected_ids: Vec<String>, // NEW - IDs not in index
    pub precision_at_k: f64,
    pub recall_at_k: f64,
    pub mrr: f64,                          // NEW
    pub source: EvalCaseSource,            // NEW - builtin or json
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EvalCaseSource {
    Builtin,
    Json,
}
```

### Task 3: Core eval_case Function with MRR Calculation

```rust
pub async fn eval_case<Lm, Ll, V>(
    engine: &Stage0Engine,
    local_mem: &Lm,
    llm: &Ll,
    vector: Option<&V>,
    case: &EvalCase,
    default_k: usize,
    source: EvalCaseSource,
) -> Result<EvalResult, Stage0Error>
where
    Lm: LocalMemoryClient,
    Ll: LlmClient,
    V: VectorBackend,
{
    let k = case.k.unwrap_or(default_k);

    // Build DCC context and call compile_context
    // (code from existing DCC integration)

    // Extract IDs by lane
    let top_ids = match case.lane {
        EvalLane::Memory => dcc_res.memories_used.iter().take(k).cloned().collect(),
        EvalLane::Code => dcc_res.code_candidates.iter().take(k).map(|c| c.id.clone()).collect(),
    };

    // Check which expected IDs are present in index
    let (present, missing): (Vec<_>, Vec<_>) = case
        .expected_ids
        .iter()
        .cloned()
        .partition(|id| index_contains_id(id, &case.lane, vector));

    // Compute metrics on present subset only
    let expected: HashSet<_> = present.iter().cloned().collect();
    let hits: Vec<_> = top_ids.iter().filter(|id| expected.contains(*id)).cloned().collect();

    let tp = hits.len() as f64;
    let k_f = k as f64;
    let denom_expected = expected.len().max(1) as f64;

    let precision_at_k = tp / k_f;
    let recall_at_k = tp / denom_expected;

    // MRR: 1/rank of first hit (0 if no hits)
    let mrr = top_ids.iter().enumerate()
        .find(|(_, id)| expected.contains(*id))
        .map(|(idx, _)| 1.0 / (idx + 1) as f64)
        .unwrap_or(0.0);

    Ok(EvalResult {
        case_id: case.id.clone(),
        lane: case.lane.clone(),
        k,
        top_ids,
        expected_ids: case.expected_ids.clone(),
        hits,
        missing_expected_ids: missing,
        precision_at_k,
        recall_at_k,
        mrr,
        source,
    })
}
```

### Task 4: Built-in Code Eval Cases

```rust
pub fn builtin_code_eval_cases() -> Vec<EvalCase> {
    vec![
        EvalCase {
            id: "code_stage0_handle_spec_auto".into(),
            description: "Stage0 should surface handle_spec_auto in code lane for speckit.auto".into(),
            lane: EvalLane::Code,
            spec_id: "SPEC-KIT-102".into(),
            spec_content: "Integrate Stage0Engine::run_stage0 into handle_spec_auto in the /speckit.auto pipeline...".into(),
            expected_ids: vec![
                "code:tui/src/chatwidget/spec_kit/pipeline_coordinator.rs::handle_spec_auto".into()
            ],
            k: Some(5),
        },
        EvalCase {
            id: "code_stage0_spec_auto_state".into(),
            description: "Stage0 should surface SpecAutoState when spec mentions Stage 0 fields".into(),
            lane: EvalLane::Code,
            spec_id: "SPEC-KIT-102".into(),
            spec_content: "Extend SpecAutoState with stage0_result, stage0_config, stage0_skip_reason...".into(),
            expected_ids: vec![
                "code:tui/src/chatwidget/spec_kit/state.rs::SpecAutoState".into()
            ],
            k: Some(5),
        },
        EvalCase {
            id: "code_stage0_dcc".into(),
            description: "Stage0 should surface compile_context for DCC-related specs".into(),
            lane: EvalLane::Code,
            spec_id: "SPEC-KIT-102".into(),
            spec_content: "The Divine Context Compiler (DCC) compile_context function...".into(),
            expected_ids: vec![
                "code:stage0/src/dcc.rs::compile_context".into()
            ],
            k: Some(5),
        },
    ]
}
```

### Task 5: External JSON Case Loader with Merging

```rust
pub fn load_eval_cases_from_file(path: &Path) -> Result<Vec<EvalCase>, Stage0Error> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Stage0Error::Internal { message: format!("Failed to read eval cases: {}", e), source: None })?;
    let cases: Vec<EvalCase> = serde_json::from_str(&text)
        .map_err(|e| Stage0Error::Internal { message: format!("Failed to parse eval cases: {}", e), source: None })?;
    Ok(cases)
}

pub fn combined_cases(
    use_builtins: bool,
    maybe_external: Option<&Path>,
) -> Result<Vec<(EvalCase, EvalCaseSource)>, Stage0Error> {
    let mut cases = Vec::new();

    if use_builtins {
        for case in builtin_code_eval_cases() {
            cases.push((case, EvalCaseSource::Builtin));
        }
        // Also include existing memory builtins if any
        for case in builtin_memory_eval_cases() {
            cases.push((case, EvalCaseSource::Builtin));
        }
    }

    if let Some(p) = maybe_external {
        let external = load_eval_cases_from_file(p)?;
        for case in external {
            cases.push((case, EvalCaseSource::Json));
        }
    }

    Ok(cases)
}
```

### Task 6: Extend /stage0.eval-backend CLI

**File:** `tui/src/chatwidget/spec_kit/commands/special.rs`

Update `Stage0EvalBackendCommand` to:

1. Add `--lane={memory,code,both}` flag (default: both)
2. Add `--strict` flag for CI mode
3. Filter cases by lane
4. Show lane and source in output

```text
case_id                              lane    source   k   P@k   R@k   MRR
code_stage0_handle_spec_auto         Code    builtin  5   1.00  1.00  1.00
code_stage0_spec_auto_state          Code    builtin  5   0.80  1.00  0.50
stage0_handle_spec_auto_memory       Memory  json     5   0.60  0.60  0.33
```

### Task 7: Add /stage0.eval-code Command

**File:** `tui/src/chatwidget/spec_kit/commands/special.rs`

New command that is sugar for `--lane=code`:

```rust
pub struct Stage0EvalCodeCommand;

impl SpecKitCommand for Stage0EvalCodeCommand {
    fn name(&self) -> &'static str {
        "stage0.eval-code"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "evaluate code lane retrieval quality (shortcut for eval-backend --lane=code)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // Parse args but force lane=Code
        // Delegate to eval_backend_impl with lane=Code preset
        // Default k=10
    }

    fn requires_args(&self) -> bool {
        false
    }
}
```

### Task 8: Tests

**File:** `stage0/src/eval.rs` (tests module)

1. Test `EvalCase` JSON parsing with lane field
2. Test `eval_case` for Memory vs Code lanes
3. Test MRR calculation with various rank positions
4. Test missing_expected_ids handling
5. Test combined_cases merging builtin + external

**File:** `tui/tests/stage0_eval_tests.rs` (new)

1. Integration test with mock backends
2. Test `/stage0.eval-code` command output

---

## Out of Scope for P86

- Learned routing/scoring for code lane (defer to P87+)
- Metrics crate integration
- NotebookLM integration for eval cases
- Incremental index updates

---

## Success Criteria

1. `/stage0.eval-backend --lane=code` runs with built-in cases and reports P@K, R@K, MRR
2. `/stage0.eval-code` command works as shortcut
3. JSON output includes all new fields (lane, source, mrr, missing_expected_ids)
4. `--strict` mode fails on missing expected IDs
5. All tests pass: 113+ stage0 + 507+ TUI

---

## Key Files Reference

- `stage0/src/eval.rs` - Core eval types and functions (may need creation)
- `stage0/src/lib.rs` - Export eval module
- `stage0/src/dcc.rs` - CompileContextResult already has code_candidates
- `tui/src/chatwidget/spec_kit/commands/special.rs` - CLI commands
- `tui/src/chatwidget/spec_kit/command_registry.rs` - Register new command
