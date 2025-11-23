# Multi-Agent Consensus System

## Overview

The multi-agent consensus system is the core differentiator of Spec-Kit. Instead of relying on a single AI model's output, Spec-Kit spawns multiple agents that independently analyze the same problem, then synthesizes their outputs into a higher-quality result.

## Why Multi-Agent?

### Single Model Problems

- **Hallucination**: One model's confident incorrect answer
- **Blind spots**: Each model has different weaknesses
- **Inconsistency**: Same model gives different answers on retry
- **Limited perspective**: One architectural approach

### Consensus Benefits

- **Error detection**: Disagreements reveal potential issues
- **Diverse perspectives**: Different models suggest different approaches
- **Higher confidence**: Agreement across models = more reliable
- **Better coverage**: Models catch each other's blind spots

## Agent Roster

### Tier 2 Agents (Multi-Agent Stages)

| Agent | Model | Cost/1M tokens | Role |
|-------|-------|----------------|------|
| `gemini-flash` | Gemini 2.5 Flash | $0.075 | Broad analysis, research |
| `claude-haiku` | Claude 3.5 Haiku | $0.25 | Edge cases, validation |
| `gpt5-medium` | GPT-5 (medium) | $0.35/run | Aggregation, synthesis |

### Tier 3 Agents (Premium Stages)

| Agent | Model | Cost | Role |
|-------|-------|------|------|
| `gemini-pro` | Gemini 2.5 Pro | $0.80/run | Critical reasoning |
| `claude-sonnet` | Claude 4.5 Sonnet | $0.80/run | Security analysis |
| `gpt5-high` | GPT-5 (high effort) | $0.80/run | Final decisions |

### Specialized Agents

| Agent | Model | Cost | Role |
|-------|-------|------|------|
| `gpt_codex` | GPT-5-Codex (HIGH) | $0.11/run | Code generation |
| `gpt5-low` | GPT-5 (low effort) | $0.10/run | Simple tasks |
| `code` | Native Rust | $0 | Heuristics, validation |

## Consensus Workflow

### Step 1: Agent Selection

Based on stage complexity and criticality:

```
/speckit.plan (Tier 2)
  → [gemini-flash, claude-haiku, gpt5-medium]

/speckit.implement (Tier 2, specialized)
  → [gpt_codex (primary), claude-haiku (validator)]

/speckit.audit (Tier 3)
  → [gemini-pro, claude-sonnet, gpt5-high]
```

### Step 2: Parallel Execution

All agents execute simultaneously:

```
Time: 0 min
├─ Gemini Flash starts reading spec.md
├─ Claude Haiku starts reading spec.md
└─ GPT-5 Medium starts reading spec.md

Time: 5 min
├─ Gemini Flash: analyzing requirements...
├─ Claude Haiku: checking edge cases...
└─ GPT-5 Medium: structuring output...

Time: 10 min
├─ Gemini Flash: COMPLETE → plan proposal
├─ Claude Haiku: COMPLETE → plan proposal
└─ GPT-5 Medium: COMPLETE → plan proposal
```

### Step 3: Result Collection

Each agent produces structured output:

```json
{
  "agent": "gemini-flash",
  "stage": "plan",
  "spec_id": "SPEC-KIT-065",
  "output": {
    "plan": "## Work Breakdown\n1. Create OAuth config...",
    "risks": ["Token expiry handling", "Provider differences"],
    "confidence": 0.85
  },
  "metrics": {
    "tokens_in": 2048,
    "tokens_out": 1024,
    "duration_ms": 45000
  }
}
```

### Step 4: Agreement Analysis

Determine level of consensus:

**Unanimous (3/3)**
```
Gemini:  "Use encrypted vault for tokens"
Claude:  "Use encrypted vault for tokens"
GPT-5:   "Use encrypted vault for tokens"
→ High confidence, auto-apply
```

**Majority (2/3)**
```
Gemini:  "Use encrypted vault"
Claude:  "Use encrypted vault"
GPT-5:   "Use database with encryption"
→ Medium confidence, apply with validation
```

**Disagreement (No majority)**
```
Gemini:  "Use encrypted vault"
Claude:  "Use HSM"
GPT-5:   "Use cloud KMS"
→ Escalate to user with all options
```

### Step 5: Synthesis

Blend agent outputs into final result:

**For Planning**:
- Combine architectural suggestions
- Merge risk analyses
- Unify work breakdown
- Note disagreements and resolutions

**For Code Generation**:
- Primary code from gpt_codex
- Validation notes from claude-haiku
- Edge cases marked for attention

**For Validation**:
- Comprehensive test scenarios from all agents
- Coverage analysis merged
- Missing tests identified

## Agreement Calculations

### Semantic Similarity

Not just string matching - uses semantic analysis:

```rust
fn calculate_agreement(results: &[AgentResult]) -> Agreement {
    // Extract key decisions from each result
    let decisions: Vec<Vec<Decision>> = results
        .iter()
        .map(|r| extract_decisions(r))
        .collect();

    // Compare decisions pairwise
    let mut agreement_score = 0.0;
    for i in 0..decisions.len() {
        for j in i+1..decisions.len() {
            agreement_score += semantic_similarity(&decisions[i], &decisions[j]);
        }
    }

    // Normalize
    let pairs = (decisions.len() * (decisions.len() - 1)) / 2;
    let normalized = agreement_score / pairs as f64;

    match normalized {
        x if x > 0.85 => Agreement::Unanimous,
        x if x > 0.60 => Agreement::Majority,
        _ => Agreement::NoConsensus,
    }
}
```

### Weighted by Confidence

Agents report confidence scores:

```rust
fn weighted_synthesis(results: &[AgentResult]) -> Synthesis {
    let total_confidence: f64 = results.iter().map(|r| r.confidence).sum();

    let weighted_output = results
        .iter()
        .map(|r| {
            let weight = r.confidence / total_confidence;
            weight_content(&r.output, weight)
        })
        .fold(String::new(), |acc, s| acc + &s);

    Synthesis { content: weighted_output, confidence: total_confidence / results.len() as f64 }
}
```

## Synthesis Strategies

### Plan Synthesis

For `/speckit.plan`, blend architectural approaches:

```markdown
## Work Breakdown (Consensus)

### From All Agents
1. Create OAuth configuration structure
2. Implement token storage mechanism
3. Add OAuth callback handler

### Gemini Flash Addition
4. Add rate limiting for auth endpoints

### Claude Haiku Addition
5. Implement token rotation for long-lived sessions

### Resolution Notes
- Token storage: Vault chosen over database (2/3 agreement)
- Rate limiting: Added based on security best practice
```

### Code Synthesis

For `/speckit.implement`, primary + validator:

```rust
// Primary implementation from gpt_codex
pub struct OAuthConfig {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

// Validator notes from claude-haiku:
// ✓ Secrets should not be logged
// ✓ Consider adding token expiry field
// ⚠ Missing: Rate limit configuration
```

### Validation Synthesis

For `/speckit.validate`, comprehensive test coverage:

```markdown
## Test Scenarios (Merged)

### Happy Path (All Agents)
- Valid OAuth login flow
- Token refresh success
- Logout clears session

### Edge Cases (Claude Haiku)
- Expired token during request
- Concurrent refresh attempts
- Malformed callback parameters

### Error Scenarios (Gemini Flash)
- Invalid client credentials
- Network timeout during OAuth
- Provider rate limiting
```

## Degradation Handling

### Agent Failure Retry

```rust
async fn execute_with_retry(agent: &SpecAgent) -> Result<AgentResult> {
    let mut attempts = 0;

    loop {
        match execute_agent(agent).await {
            Ok(result) if !result.is_empty() => return Ok(result),
            Ok(_) => {
                // Empty result - retry with guidance
                attempts += 1;
                if attempts >= 3 {
                    return Err(Error::EmptyResult);
                }
                // Add storage guidance to prompt
                continue;
            }
            Err(e) => {
                attempts += 1;
                if attempts >= 3 {
                    return Err(e);
                }
                // Exponential backoff
                sleep(Duration::from_secs(2_u64.pow(attempts))).await;
            }
        }
    }
}
```

### Graceful Degradation

```rust
fn handle_degradation(results: Vec<Result<AgentResult>>) -> ConsensusResult {
    let successful: Vec<_> = results.into_iter().filter_map(|r| r.ok()).collect();

    match successful.len() {
        3 => {
            // Full consensus
            synthesize_full(successful)
        }
        2 => {
            // 2/3 consensus still valid
            log::warn!("Degraded consensus: 1 agent failed");
            synthesize_degraded(successful)
        }
        1 => {
            // Single agent fallback
            log::error!("Minimal consensus: 2 agents failed");
            ConsensusResult::degraded(successful[0].clone())
        }
        0 => {
            // Complete failure
            ConsensusResult::failed("All agents failed")
        }
    }
}
```

## MCP Integration

### Performance

- **Subprocess** (old): ~50ms per operation
- **Native MCP** (new): ~8.7ms per operation (5.3x faster)

### Data Flow

```
Agent Output
    ↓
[MCP Client] → Structured message
    ↓
[local-memory] → Store in knowledge base
    ↓
[consensus_db] → Store in SQLite
    ↓
[Evidence] → Write to filesystem
```

### Memory Storage

Agent outputs stored in local-memory for retrieval:

```rust
// Store agent result
mcp_client.store_memory(StoreRequest {
    content: agent_result.output,
    domain: "consensus",
    tags: vec![
        format!("spec:{}", spec_id),
        format!("stage:{}", stage),
        format!("agent:{}", agent.name()),
    ],
    importance: 8,
}).await?;
```

## Evidence Collection

### Per-Agent Artifacts

```
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-065/
├── plan-gemini-2025-10-27T10:15:00Z.json
│   {
│     "agent": "gemini-flash",
│     "model": "gemini-2.5-flash",
│     "input": "...",
│     "output": "...",
│     "tokens": { "in": 2048, "out": 1024 },
│     "duration_ms": 45000,
│     "cost_usd": 0.008
│   }
├── plan-claude-2025-10-27T10:15:00Z.json
├── plan-gpt_pro-2025-10-27T10:16:00Z.json
└── plan-synthesis-2025-10-27T10:18:00Z.json
    {
      "agreement": "majority",
      "confidence": 0.78,
      "content": "...",
      "agent_contributions": [...]
    }
```

### Telemetry

```json
{
  "command": "speckit.plan",
  "spec_id": "SPEC-KIT-065",
  "stage": "plan",
  "agents": [
    { "agent": "gemini-flash", "status": "completed", "cost": 0.008 },
    { "agent": "claude-haiku", "status": "completed", "cost": 0.012 },
    { "agent": "gpt5-medium", "status": "completed", "cost": 0.35 }
  ],
  "consensus": {
    "agreement": "majority",
    "confidence": 0.78
  },
  "total_cost": 0.37,
  "duration_ms": 720000
}
```

## Conflict Resolution

### Auto-Resolution

When agents disagree but resolution is clear:

```rust
fn attempt_auto_resolution(conflict: &Conflict) -> Option<Resolution> {
    // Check for unanimous agreement on resolution
    if conflict.resolution_votes.iter().all(|v| v == &conflict.resolution_votes[0]) {
        return Some(Resolution::auto(conflict.resolution_votes[0].clone()));
    }

    // Check for 2/3 majority with GPT-5 validation
    let majority = find_majority(&conflict.resolution_votes);
    if let Some(resolution) = majority {
        if gpt5_validates(&resolution) {
            return Some(Resolution::validated(resolution));
        }
    }

    // Cannot auto-resolve
    None
}
```

### User Escalation

When agents fundamentally disagree:

```
⚠ CONSENSUS CONFLICT: Token Storage Approach

Agent perspectives:
1. Gemini Flash: "Use HashiCorp Vault for enterprise-grade security"
2. Claude Haiku: "Use Hardware Security Module (HSM) for compliance"
3. GPT-5 Medium: "Use cloud KMS (AWS/GCP) for managed solution"

Analysis:
- Security: All approaches adequate
- Complexity: Vault > HSM > KMS
- Cost: HSM > Vault > KMS
- Compliance: HSM best for regulated industries

Please choose an approach or provide additional context:
>
```

## Performance Metrics

### Typical Timings

| Stage | Agents | Time |
|-------|--------|------|
| Plan | 3 parallel | 10-12 min |
| Tasks | 1 | 3-5 min |
| Implement | 2 parallel | 8-12 min |
| Validate | 3 parallel | 10-12 min |
| Audit | 3 parallel | 10-12 min |
| Unlock | 3 parallel | 10-12 min |

### Cost Breakdown

| Stage | Agents | Cost |
|-------|--------|------|
| Plan | gemini + claude + gpt5 | $0.35 |
| Tasks | gpt5-low | $0.10 |
| Implement | codex + claude | $0.11 |
| Validate | gemini + claude + gpt5 | $0.35 |
| Audit | 3 premium | $0.80 |
| Unlock | 3 premium | $0.80 |
| **Total** | | **$2.51** |

(+ quality gates: ~$0 native + occasional $0.10 validation = ~$2.70)
