# Spec-Kit Command Reference

Complete reference for all 13 /speckit.* commands.

---

## Overview

**Spec-Kit Framework** provides 13 commands organized by tier:
- **Tier 0** (Native): FREE, instant (<1s)
- **Tier 1** (Single Agent): ~$0.10, 3-5 min
- **Tier 2** (Multi-Agent): ~$0.35, 8-12 min
- **Tier 3** (Premium): ~$0.80, 10-12 min
- **Tier 4** (Full Pipeline): ~$2.70, 45-50 min

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/commands/`

---

## Command Quick Reference

| Command | Tier | Cost | Time | Purpose |
|---------|------|------|------|---------|
| `/speckit.new` | 0 (Native) | $0 | <1s | Create SPEC |
| `/speckit.specify` | 1 (Single) | ~$0.10 | 3-5min | Draft PRD |
| `/speckit.clarify` | 0 (Native) | $0 | <1s | Detect ambiguity |
| `/speckit.analyze` | 0 (Native) | $0 | <1s | Check consistency |
| `/speckit.checklist` | 0 (Native) | $0 | <1s | Quality scoring |
| `/speckit.plan` | 2 (Multi) | ~$0.35 | 10-12min | Work breakdown |
| `/speckit.tasks` | 1 (Single) | ~$0.10 | 3-5min | Task decomposition |
| `/speckit.implement` | 2 (Code) | ~$0.11 | 8-12min | Code generation |
| `/speckit.validate` | 2 (Multi) | ~$0.35 | 10-12min | Test strategy |
| `/speckit.audit` | 3 (Premium) | ~$0.80 | 10-12min | Compliance check |
| `/speckit.unlock` | 3 (Premium) | ~$0.80 | 10-12min | Ship decision |
| `/speckit.auto` | 4 (Pipeline) | ~$2.70 | 45-50min | Full automation |
| `/speckit.status` | 0 (Native) | $0 | <1s | Status dashboard |

---

## Tier 0: Native Commands (FREE)

### /speckit.new

**Purpose**: Create new SPEC with template

**Tier**: 0 (Native, zero agents)
**Cost**: $0
**Time**: <1 second
**Agent Count**: 0

**Usage**:
```
/speckit.new <description>
```

**Examples**:
```
/speckit.new Add OAuth2 authentication with JWT tokens

/speckit.new Implement rate limiting for API endpoints using token bucket algorithm

/speckit.new Create user dashboard with activity metrics and export functionality
```

**What It Does**:
1. Generates unique SPEC-ID (e.g., SPEC-KIT-125)
2. Creates directory: `docs/SPEC-KIT-125-<slug>/`
3. Generates spec.md from template with:
   - Description as title
   - Empty objectives/scope/deliverables sections
   - Created timestamp
4. Creates subdirectories:
   - `evidence/` (for artifacts)
   - `adr/` (for architectural decisions)
5. Updates `SPEC.md` task tracker
6. Returns SPEC-ID to user

**Output**:
```
✅ Created SPEC-KIT-125: Add OAuth2 authentication with JWT tokens

Directory: docs/SPEC-KIT-125-add-oauth2-authentication-jwt/
Files created:
- spec.md (template)
- evidence/ (directory)
- adr/ (directory)

Next steps:
- Run /speckit.specify SPEC-KIT-125 to draft comprehensive PRD
- Or run /speckit.auto SPEC-KIT-125 for full automation
```

**Implementation**: `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs`

**No AI**: Uses template system and native SPEC-ID generation

---

### /speckit.clarify

**Purpose**: Detect ambiguities, vague language, missing details

**Tier**: 0 (Native heuristics)
**Cost**: $0
**Time**: <1 second
**Agent Count**: 0

**Usage**:
```
/speckit.clarify <SPEC-ID>
```

**Examples**:
```
/speckit.clarify SPEC-KIT-125
```

**What It Does**:
1. Reads spec.md
2. Runs heuristic pattern matching:
   - **Vague language**: "maybe", "probably", "should", "could"
   - **Undefined terms**: References without definitions
   - **Missing sections**: Empty objectives/scope/deliverables
   - **Ambiguous requirements**: "fast", "scalable", without metrics
3. Generates report with line numbers
4. Suggests improvements

**Output**:
```
🔍 Ambiguity Report: SPEC-KIT-125

Vague Language (3 issues):
├─ Line 12: "should be fast" → Specify target latency (e.g., <100ms p95)
├─ Line 28: "probably need caching" → Confirm requirement or remove
└─ Line 45: "could support OAuth2" → Required or optional?

Missing Details (2 issues):
├─ Section "Success Criteria" is empty
└─ Section "Acceptance Criteria" is empty

Undefined Terms (1 issue):
└─ "JWT refresh flow" referenced but not defined

Recommendations:
1. Add quantitative metrics for performance requirements
2. Define all technical terms in Glossary section
3. Fill in Success Criteria and Acceptance Criteria
4. Replace modal language (should/could) with definitive statements

Quality Score: 6/10 (needs improvement)
```

**Implementation**: `codex-rs/tui/src/chatwidget/spec_kit/clarify_native.rs`

**Pattern Matching**:
```rust
const VAGUE_PATTERNS: &[&str] = &[
    "maybe", "probably", "should", "could", "might",
    "fast", "slow", "big", "small", "scalable",
    "efficient", "performant", "optimized",
];
```

---

### /speckit.analyze

**Purpose**: Consistency checking (structural diff)

**Tier**: 0 (Native)
**Cost**: $0
**Time**: <1 second
**Agent Count**: 0

**Usage**:
```
/speckit.analyze <SPEC-ID>
```

**Examples**:
```
/speckit.analyze SPEC-KIT-125
```

**What It Does**:
1. Reads spec.md, plan.md, tasks.md
2. Structural validation:
   - **ID consistency**: SPEC-ID matches in all files
   - **Cross-references**: All references valid
   - **Section coverage**: Required sections present
   - **Deliverable tracking**: All deliverables in tasks
3. Generates consistency report

**Output**:
```
📊 Consistency Analysis: SPEC-KIT-125

ID Consistency: ✅ PASS
├─ spec.md: SPEC-KIT-125
├─ plan.md: SPEC-KIT-125
└─ tasks.md: SPEC-KIT-125

Cross-References: ⚠️ ISSUES (2)
├─ spec.md line 34 references "ARCH-002" (not found)
└─ plan.md line 67 references deliverable "oauth-flow.md" (not in spec)

Section Coverage: ✅ PASS
├─ Objectives: Present
├─ Scope: Present
├─ Deliverables: Present (4 items)
└─ Success Criteria: Present

Deliverable Tracking: ⚠️ ISSUES (1)
└─ Deliverable "token-refresh.md" in spec but missing from tasks.md

Recommendations:
1. Fix broken reference to ARCH-002 or remove
2. Add "oauth-flow.md" to deliverables list
3. Add task for "token-refresh.md" implementation

Consistency Score: 7/10 (minor issues)
```

**Implementation**: `codex-rs/tui/src/chatwidget/spec_kit/analyze_native.rs`

---

### /speckit.checklist

**Purpose**: Quality rubric scoring

**Tier**: 0 (Native)
**Cost**: $0
**Time**: <1 second
**Agent Count**: 0

**Usage**:
```
/speckit.checklist <SPEC-ID>
```

**Examples**:
```
/speckit.checklist SPEC-KIT-125
```

**What It Does**:
1. Evaluates spec against quality rubric:
   - **Completeness**: All sections filled
   - **Clarity**: Specific language, defined terms
   - **Testability**: Measurable success criteria
   - **Consistency**: No contradictions
2. Calculates scores (0-10 per category)
3. Overall grade (A-F)

**Output**:
```
📋 Quality Checklist: SPEC-KIT-125

Completeness (7/10):
├─ ✅ Title and description present
├─ ✅ Objectives defined (3 objectives)
├─ ✅ Scope (in/out) defined
├─ ✅ Deliverables listed (4 deliverables)
├─ ⚠️ Success criteria partially defined (missing metrics)
└─ ❌ Acceptance criteria empty

Clarity (6/10):
├─ ✅ Technical terms defined (OAuth2, JWT)
├─ ⚠️ Some vague language ("fast", "scalable")
└─ ❌ Missing quantitative metrics

Testability (5/10):
├─ ⚠️ Success criteria present but not measurable
├─ ❌ No test strategy defined
└─ ❌ Acceptance criteria empty

Consistency (8/10):
├─ ✅ No contradictions found
├─ ✅ Cross-references valid
└─ ⚠️ Minor: deliverable "token-refresh.md" not in tasks

Overall Score: 6.5/10 (Grade: C)

Recommendations:
1. Add quantitative metrics to success criteria
2. Define acceptance criteria with test cases
3. Replace vague language with specific terms
4. Add test strategy section

Next Steps:
- Fix issues and re-run /speckit.checklist
- Or proceed with /speckit.auto (quality gates will catch issues)
```

**Implementation**: `codex-rs/tui/src/chatwidget/spec_kit/checklist_native.rs`

---

### /speckit.status

**Purpose**: Status dashboard (TUI widget)

**Tier**: 0 (Native)
**Cost**: $0
**Time**: <1 second
**Agent Count**: 0

**Usage**:
```
/speckit.status <SPEC-ID>
```

**Examples**:
```
/speckit.status SPEC-KIT-125
```

**What It Does**:
1. Reads workflow state
2. Displays TUI dashboard with:
   - Stage completion (checkmarks)
   - Artifacts generated
   - Evidence paths
   - Quality gate status
   - Cost tracking

**Output** (TUI widget):
```
╭─────────────────────────────────────────────────────────────╮
│ SPEC-KIT-125: Add OAuth2 authentication with JWT tokens    │
├─────────────────────────────────────────────────────────────┤
│ Stages:                                                     │
│ ✅ new      (native, $0)                                    │
│ ✅ specify  (1 agent, $0.10, 4m 23s)                        │
│ ✅ clarify  (native, $0)                                    │
│ ✅ analyze  (native, $0)                                    │
│ ✅ checklist (native, $0)                                   │
│ ✅ plan     (3 agents, $0.35, 11m 45s)                      │
│ ✅ tasks    (1 agent, $0.10, 3m 56s)                        │
│ 🔄 implement (in progress, 2 agents, est. $0.11)            │
│ ⏳ validate  (pending)                                      │
│ ⏳ audit     (pending)                                      │
│ ⏳ unlock    (pending)                                      │
├─────────────────────────────────────────────────────────────┤
│ Artifacts:                                                  │
│ ├─ spec.md (2.3 KB)                                         │
│ ├─ plan.md (5.7 KB)                                         │
│ ├─ tasks.md (3.2 KB)                                        │
│ └─ evidence/ (12 files, 450 KB)                             │
├─────────────────────────────────────────────────────────────┤
│ Quality Gates:                                              │
│ ├─ Clarify: ✅ PASS (3 issues fixed)                        │
│ ├─ Analyze: ✅ PASS (no contradictions)                     │
│ └─ Checklist: ⚠️ 6.5/10 (Grade C, acceptable)              │
├─────────────────────────────────────────────────────────────┤
│ Cost: $0.65 / $2.70 estimated total                         │
│ Time: 19m 24s / ~50m estimated total                        │
╰─────────────────────────────────────────────────────────────╯

Press 'q' to close, 'r' to refresh
```

**Implementation**: `codex-rs/tui/src/chatwidget/spec_kit/command_handlers.rs` (status_command)

---

## Tier 1: Single-Agent Commands

### /speckit.specify

**Purpose**: Draft/refine PRD with strategic analysis

**Tier**: 1 (Single Agent)
**Cost**: ~$0.10
**Time**: 3-5 minutes
**Agent**: `gpt-5-low` (strategic reasoning)

**Usage**:
```
/speckit.specify <SPEC-ID> [additional context]
```

**Examples**:
```
/speckit.specify SPEC-KIT-125

/speckit.specify SPEC-KIT-125 Focus on security and OWASP top 10 compliance
```

**What It Does**:
1. Reads initial spec.md
2. Spawns `gpt-5-low` agent with PRD template
3. Agent analyzes and expands:
   - **Objectives**: Clear, measurable goals
   - **Scope**: Detailed in/out boundaries
   - **Deliverables**: Concrete artifacts
   - **Success Criteria**: Quantitative metrics
   - **Risks**: Potential blockers
4. Writes refined spec.md

**Output**:
```
📝 PRD Refinement (1 agent: gpt-5-low)

Agent: gpt-5-low (strategic analysis)
Time: 4m 12s
Cost: $0.09

Changes to spec.md:
├─ Expanded Objectives (3 → 5 objectives)
├─ Detailed Scope section (+800 words)
├─ Added Deliverables (4 concrete artifacts)
├─ Success Criteria with metrics (p95 latency <100ms, etc.)
├─ Risk Analysis (3 risks identified)
└─ Acceptance Criteria (8 test scenarios)

Quality Score: 8.5/10 (improved from 6.5/10)

spec.md updated. Next: /speckit.plan SPEC-KIT-125
```

**Configuration**:
```toml
# ~/.code/config.toml

[quality_gates]
specify = ["code"]  # Single agent (default: gpt-5-low)
```

---

### /speckit.tasks

**Purpose**: Task decomposition from plan

**Tier**: 1 (Single Agent)
**Cost**: ~$0.10
**Time**: 3-5 minutes
**Agent**: `gpt-5-low`

**Usage**:
```
/speckit.tasks <SPEC-ID>
```

**Examples**:
```
/speckit.tasks SPEC-KIT-125
```

**What It Does**:
1. Reads plan.md
2. Spawns `gpt-5-low` for structured breakdown
3. Agent generates:
   - Task list with IDs
   - Dependencies
   - Effort estimates
   - Assignable units
4. Writes tasks.md
5. Updates SPEC.md task tracker

**Output**:
```
📋 Task Decomposition (1 agent: gpt-5-low)

Agent: gpt-5-low
Time: 3m 45s
Cost: $0.08

Generated tasks.md with 12 tasks:
├─ T1: Setup OAuth2 provider configuration (2h)
├─ T2: Implement JWT token generation (3h)
├─ T3: Create token validation middleware (4h)
├─ T4: Implement refresh token flow (5h)
├─ T5: Add user session management (3h)
├─ T6: Create login/logout endpoints (2h)
├─ T7: Implement authorization guards (4h)
├─ T8: Add rate limiting (3h)
├─ T9: Write unit tests for token logic (4h)
├─ T10: Write integration tests for auth flow (5h)
├─ T11: Add security audit tests (3h)
└─ T12: Document OAuth2 setup guide (2h)

Total effort: 40 hours
Critical path: T2 → T3 → T4 → T10

SPEC.md task tracker updated.
Next: /speckit.implement SPEC-KIT-125
```

---

## Tier 2: Multi-Agent Commands

### /speckit.plan

**Purpose**: Work breakdown with multi-agent consensus

**Tier**: 2 (Multi-Agent)
**Cost**: ~$0.35
**Time**: 10-12 minutes
**Agents**: 3 (gemini-flash, claude-haiku, gpt-5-medium)

**Usage**:
```
/speckit.plan <SPEC-ID> [context]
```

**Examples**:
```
/speckit.plan SPEC-KIT-125

/speckit.plan SPEC-KIT-125 Consider microservices architecture
```

**What It Does**:
1. Reads spec.md
2. Spawns 3 agents concurrently
3. Each agent proposes plan independently
4. Consensus coordinator synthesizes:
   - Agreed approach (unanimous)
   - Points of disagreement
   - Recommended path (majority or best)
5. Writes plan.md

**Output**:
```
📋 Multi-Agent Planning (3 agents: gemini, claude, gpt-5)

Agents:
├─ gemini-flash (completed in 9m 23s)
├─ claude-haiku (completed in 10m 45s)
└─ gpt-5-medium (completed in 11m 12s)

Consensus: 3/3 agents

Agreed Approach:
├─ Use existing OAuth2 library (not build from scratch)
├─ JWT with RS256 signing algorithm
├─ Refresh token rotation for security
├─ Redis for session storage
└─ Rate limiting per user

Points of Disagreement:
├─ Gemini: Suggested immediate token expiry (15min)
├─ Claude: Recommended longer expiry (1h) with refresh
└─ GPT-5: Proposed configurable expiry (default 30min)

Recommended: Configurable expiry (2 agents in favor)

Work Breakdown:
1. OAuth2 Provider Integration (Gemini's approach)
2. JWT Token Service (Claude's implementation pattern)
3. Session Management (GPT-5's Redis strategy)
4. Rate Limiting (Consensus: token bucket algorithm)
5. Security Audit (All agents agree: OWASP checklist)

plan.md created (5.7 KB)
Cost: $0.34
Time: 11m 45s

Next: /speckit.tasks SPEC-KIT-125
```

**Configuration**:
```toml
[quality_gates]
plan = ["gemini", "claude", "code"]  # 3 agents (balanced)
# or
plan = ["gemini", "gemini", "gemini"]  # Cheap ($0.10 total)
# or
plan = ["gemini-pro", "claude-opus", "gpt-5"]  # Premium ($1.20 total)
```

---

### /speckit.implement

**Purpose**: Code generation with specialist model

**Tier**: 2 (Specialist + Validator)
**Cost**: ~$0.11
**Time**: 8-12 minutes
**Agents**: 2 (gpt-5-codex HIGH, claude-haiku validator)

**Usage**:
```
/speckit.implement <SPEC-ID>
```

**Examples**:
```
/speckit.implement SPEC-KIT-125
```

**What It Does**:
1. Reads plan.md and tasks.md
2. Spawns `gpt-5-codex` (HIGH reasoning) for code generation
3. Spawns `claude-haiku` for validation
4. Code generation:
   - Implements all deliverables
   - Adds comprehensive docstrings
   - Includes type hints
   - Follows project conventions
5. Validation:
   - Checks code quality
   - Runs static analysis
   - Verifies tests compile
6. Writes code files

**Output**:
```
🔨 Code Generation (2 agents: gpt-5-codex, claude-haiku)

Agent 1: gpt-5-codex (HIGH reasoning)
└─ Generated code (12m 34s)

Files created:
├─ src/auth/oauth2_provider.rs (234 lines)
├─ src/auth/jwt_service.rs (189 lines)
├─ src/auth/session_manager.rs (156 lines)
├─ src/auth/middleware.rs (98 lines)
├─ src/auth/rate_limiter.rs (145 lines)
└─ tests/auth_integration_tests.rs (312 lines)

Agent 2: claude-haiku (validator)
└─ Validation (3m 12s)

Validation Results:
├─ ✅ cargo fmt --check (passed)
├─ ✅ cargo clippy (0 warnings)
├─ ✅ cargo build (compiled successfully)
├─ ✅ cargo test --no-run (tests compile)
└─ ✅ Code quality: 9/10

Cost: $0.11 (codex: $0.09, validator: $0.02)
Time: 15m 46s

Next: /speckit.validate SPEC-KIT-125
```

**Configuration**:
```toml
[quality_gates]
implement = ["gpt_codex", "claude"]  # Specialist + validator
# gpt_codex uses HIGH reasoning by default
```

---

### /speckit.validate

**Purpose**: Test strategy consensus

**Tier**: 2 (Multi-Agent)
**Cost**: ~$0.35
**Time**: 10-12 minutes
**Agents**: 3 (gemini-flash, claude-haiku, gpt-5-medium)

**Usage**:
```
/speckit.validate <SPEC-ID>
```

**Examples**:
```
/speckit.validate SPEC-KIT-125
```

**What It Does**:
1. Reads implementation code
2. Spawns 3 agents for test strategy
3. Each agent proposes:
   - Unit test coverage
   - Integration test scenarios
   - E2E test flows
   - Security test cases
4. Consensus on comprehensive test plan
5. Writes validation_plan.md

**Output**:
```
🧪 Test Strategy (3 agents: gemini, claude, gpt-5)

Agents:
├─ gemini-flash (completed in 10m 12s)
├─ claude-haiku (completed in 11m 34s)
└─ gpt-5-medium (completed in 10m 56s)

Consensus: 3/3 agents

Test Coverage Strategy:
├─ Unit Tests (all agents agree):
│   ├─ JWT token generation/validation
│   ├─ Session creation/retrieval
│   ├─ Rate limiter logic
│   └─ Middleware authorization
│
├─ Integration Tests (consensus):
│   ├─ Full OAuth2 flow (login → token → refresh → logout)
│   ├─ Concurrent session handling
│   ├─ Rate limit enforcement across requests
│   └─ Token expiry and refresh scenarios
│
├─ Security Tests (all agents agree):
│   ├─ OWASP A2: Broken Authentication (replay attacks, etc.)
│   ├─ OWASP A3: Sensitive Data Exposure (token leakage)
│   ├─ OWASP A5: Broken Access Control (unauthorized access)
│   └─ OWASP A7: XSS (token injection attacks)
│
└─ Performance Tests (GPT-5's addition, accepted by others):
    ├─ Token generation throughput (target: 1000/s)
    ├─ Session lookup latency (target: <10ms p95)
    └─ Rate limiter overhead (target: <1ms)

Target Coverage: 85% line coverage (all agents agree)

validation_plan.md created (4.2 KB)
Cost: $0.34
Time: 11m 34s

Next: /speckit.audit SPEC-KIT-125
```

---

## Tier 3: Premium Commands

### /speckit.audit

**Purpose**: Compliance and security validation

**Tier**: 3 (Premium Multi-Agent)
**Cost**: ~$0.80
**Time**: 10-12 minutes
**Agents**: 3 (gemini-pro, claude-sonnet, gpt-5-high)

**Usage**:
```
/speckit.audit <SPEC-ID>
```

**Examples**:
```
/speckit.audit SPEC-KIT-125
```

**What It Does**:
1. Reads all code and tests
2. Spawns 3 premium agents for deep analysis
3. Each agent audits:
   - **Security**: OWASP top 10, CWE common weaknesses
   - **Compliance**: Standards (OAuth2 RFC, JWT RFC)
   - **Quality**: Code smells, anti-patterns
   - **Performance**: Bottlenecks, scalability
4. Consensus on findings and recommendations
5. Writes audit_report.md

**Output**:
```
🔒 Security & Compliance Audit (3 agents: gemini-pro, claude-sonnet, gpt-5-high)

Agents:
├─ gemini-pro (completed in 11m 23s)
├─ claude-sonnet (completed in 10m 45s)
└─ gpt-5-high (completed in 12m 01s)

Consensus: 3/3 agents

Security Findings:
├─ ✅ OWASP A2 (Broken Auth): PASS (all agents agree)
│   └─ Proper token validation, no replay attacks
├─ ✅ OWASP A3 (Data Exposure): PASS (all agents agree)
│   └─ Tokens encrypted in transit (HTTPS), not logged
├─ ⚠️ OWASP A5 (Access Control): MINOR ISSUE (2/3 agents)
│   ├─ Claude: Missing authorization check in /refresh endpoint
│   └─ GPT-5: Agrees, suggests adding user_id validation
├─ ✅ OWASP A7 (XSS): PASS (all agents agree)
│   └─ Input sanitization present
└─ ✅ Token Security: PASS (all agents agree)
    └─ RS256 signing, proper key management

Compliance Findings:
├─ ✅ OAuth2 RFC 6749: COMPLIANT (all agents agree)
├─ ✅ JWT RFC 7519: COMPLIANT (all agents agree)
└─ ⚠️ Refresh Token Best Practices: MINOR DEVIATION (Gemini)
    └─ Recommends token rotation on each refresh

Quality Findings:
├─ ✅ Code Quality: 9/10 (consensus)
├─ ✅ Test Coverage: 87% (exceeds 85% target)
└─ ⚠️ Performance: 1 bottleneck identified
    └─ Redis session lookup could be cached (Claude's finding)

Critical Issues: 0
Major Issues: 0
Minor Issues: 3

Recommendations (Consensus):
1. Add user_id validation to /refresh endpoint (SECURITY)
2. Implement token rotation on refresh (BEST PRACTICE)
3. Add caching layer for session lookups (PERFORMANCE)

Audit Decision: ✅ PASS (with minor recommendations)

audit_report.md created (6.8 KB)
Cost: $0.78
Time: 12m 01s

Next: /speckit.unlock SPEC-KIT-125
```

---

### /speckit.unlock

**Purpose**: Final ship/no-ship decision

**Tier**: 3 (Premium Multi-Agent)
**Cost**: ~$0.80
**Time**: 10-12 minutes
**Agents**: 3 (gemini-pro, claude-sonnet, gpt-5-high)

**Usage**:
```
/speckit.unlock <SPEC-ID>
```

**Examples**:
```
/speckit.unlock SPEC-KIT-125
```

**What It Does**:
1. Reads all artifacts (spec, plan, code, tests, audit)
2. Spawns 3 premium agents for final review
3. Each agent evaluates:
   - **Completeness**: All deliverables present
   - **Quality**: Code meets standards
   - **Security**: No critical issues
   - **Readiness**: Production-ready
4. Consensus on ship/no-ship
5. Writes unlock_decision.md

**Output**:
```
🚀 Unlock Decision (3 agents: gemini-pro, claude-sonnet, gpt-5-high)

Agents:
├─ gemini-pro (completed in 10m 34s)
├─ claude-sonnet (completed in 11m 12s)
└─ gpt-5-high (completed in 10m 45s)

Consensus: 3/3 agents

Completeness Review:
├─ ✅ All deliverables present (4/4)
├─ ✅ Tests written and passing (87% coverage)
├─ ✅ Documentation complete (OAuth2 setup guide)
└─ ✅ Security audit passed

Quality Review:
├─ ✅ Code quality: 9/10
├─ ✅ Test quality: 8.5/10
├─ ✅ No critical issues
└─ ⚠️ 3 minor recommendations (non-blocking)

Security Review:
├─ ✅ OWASP top 10: PASS
├─ ✅ OAuth2/JWT compliance: PASS
└─ ⚠️ 1 minor security recommendation (token rotation)

Readiness Review:
├─ ✅ Production-ready (all agents agree)
├─ ✅ Deployment plan documented
├─ ✅ Rollback strategy defined
└─ ✅ Monitoring configured

Ship Decision:
╔════════════════════════════════════════════╗
║  ✅ SHIP APPROVED (3/3 agents)             ║
╚════════════════════════════════════════════╝

Gemini: SHIP ✅
└─ "Implementation is complete, secure, and well-tested. Minor recommendations can be addressed post-launch."

Claude: SHIP ✅
└─ "Code meets quality standards. Security audit passed with minor suggestions for improvement."

GPT-5: SHIP ✅
└─ "Production-ready. Excellent test coverage and documentation. Recommend addressing token rotation in v1.1."

Post-Launch TODO:
1. Monitor authentication latency metrics
2. Implement token rotation (v1.1)
3. Add session lookup caching (v1.1)

unlock_decision.md created (3.2 KB)
Cost: $0.79
Time: 11m 12s

🎉 SPEC-KIT-125 complete! Ready to ship.
```

---

## Tier 4: Full Pipeline

### /speckit.auto

**Purpose**: Full 6-stage automation pipeline

**Tier**: 4 (Strategic Routing)
**Cost**: ~$2.70 (75% cheaper than original $11)
**Time**: 45-50 minutes
**Stages**: specify → plan → tasks → implement → validate → audit → unlock

**Usage**:
```
/speckit.auto <SPEC-ID> [--from STAGE]
```

**Examples**:
```
/speckit.auto SPEC-KIT-125

/speckit.auto SPEC-KIT-125 --from plan  # Resume from plan stage
```

**What It Does**:
1. Runs all stages in sequence:
   - Native quality checks (FREE): clarify, analyze, checklist
   - specify (1 agent, $0.10)
   - plan (3 agents, $0.35)
   - tasks (1 agent, $0.10)
   - implement (2 agents, $0.11)
   - validate (3 agents, $0.35)
   - audit (3 premium, $0.80)
   - unlock (3 premium, $0.80)
2. Quality gates between stages
3. Auto-advancement on success
4. Stops on gate failure (manual review required)

**Output** (abbreviated):
```
🤖 Full Automation Pipeline: SPEC-KIT-125

Pipeline Stages: 8 stages (3 native + 5 multi-agent)
Estimated Cost: $2.70
Estimated Time: 45-50 minutes

[Stage 1/8] clarify (native)...
✅ Completed in <1s ($0)
Quality Gate: ✅ PASS (2 issues found, auto-fixed)

[Stage 2/8] specify (1 agent)...
Agent: gpt-5-low
✅ Completed in 4m 12s ($0.09)
Quality Gate: ✅ PASS (quality score 8.5/10)

[Stage 3/8] plan (3 agents)...
Agents: gemini, claude, gpt-5
✅ Completed in 11m 45s ($0.34)
Consensus: 3/3 agents
Quality Gate: ✅ PASS (unanimous agreement)

[Stage 4/8] tasks (1 agent)...
Agent: gpt-5-low
✅ Completed in 3m 56s ($0.08)
Quality Gate: ✅ PASS (12 tasks generated)

[Stage 5/8] implement (2 agents)...
Agents: gpt-5-codex, claude-haiku
✅ Completed in 15m 46s ($0.11)
Validation: ✅ PASS (all checks passed)
Quality Gate: ✅ PASS

[Stage 6/8] validate (3 agents)...
Agents: gemini, claude, gpt-5
✅ Completed in 11m 34s ($0.34)
Consensus: 3/3 agents
Quality Gate: ✅ PASS (85% coverage target met)

[Stage 7/8] audit (3 premium agents)...
Agents: gemini-pro, claude-sonnet, gpt-5-high
✅ Completed in 12m 01s ($0.78)
Consensus: 3/3 agents (0 critical, 0 major, 3 minor issues)
Quality Gate: ✅ PASS

[Stage 8/8] unlock (3 premium agents)...
Agents: gemini-pro, claude-sonnet, gpt-5-high
✅ Completed in 11m 12s ($0.79)
Decision: ✅ SHIP (3/3 agents approve)

╔══════════════════════════════════════════════╗
║  🎉 PIPELINE COMPLETE                        ║
╠══════════════════════════════════════════════╣
║  Total Cost: $2.73                           ║
║  Total Time: 47m 23s                         ║
║  Stages Passed: 8/8 ✅                        ║
║  Decision: SHIP APPROVED ✅                   ║
╚══════════════════════════════════════════════╝

Artifacts:
├─ spec.md (refined PRD)
├─ plan.md (consensus work breakdown)
├─ tasks.md (12 tasks)
├─ src/auth/*.rs (6 files, 1134 lines)
├─ tests/*.rs (312 lines, 87% coverage)
├─ validation_plan.md
├─ audit_report.md
└─ unlock_decision.md

Evidence: docs/SPEC-KIT-125-.../evidence/ (28 files, 2.1 MB)

Next Steps:
1. Review artifacts
2. Address 3 minor audit recommendations (optional, non-blocking)
3. Deploy to production
```

**Resumption** (if interrupted):
```
/speckit.auto SPEC-KIT-125 --from validate

Resuming from stage 6/8 (validate)...
Previous stages: specify ✅, plan ✅, tasks ✅, implement ✅
Remaining: validate, audit, unlock
```

**Configuration**:
```toml
[quality_gates]
# Customize each stage's agents
plan = ["gemini", "claude", "code"]
tasks = ["code"]
implement = ["gpt_codex", "claude"]
validate = ["gemini", "claude", "code"]
audit = ["gemini-pro", "claude-sonnet", "gpt-5"]
unlock = ["gemini-pro", "claude-sonnet", "gpt-5"]
```

---

## Legacy Commands (Backward Compatibility)

These commands still work but are deprecated:

| Legacy Command | New Command | Status |
|----------------|-------------|--------|
| `/new-spec` | `/speckit.new` | Deprecated |
| `/spec-plan` | `/speckit.plan` | Deprecated |
| `/spec-tasks` | `/speckit.tasks` | Deprecated |
| `/spec-implement` | `/speckit.implement` | Deprecated |
| `/spec-validate` | `/speckit.validate` | Deprecated |
| `/spec-audit` | `/speckit.audit` | Deprecated |
| `/spec-unlock` | `/speckit.unlock` | Deprecated |
| `/spec-auto` | `/speckit.auto` | Deprecated |
| `/spec-status` | `/speckit.status` | Deprecated |

**Migration**: Replace `/spec-*` with `/speckit.*` in all workflows

---

## Cost Summary

### Per-Command Costs

| Command | Agents | Provider(s) | Input Tokens | Output Tokens | Cost |
|---------|--------|-------------|--------------|---------------|------|
| `new` | 0 | Native | 0 | 0 | $0.00 |
| `clarify` | 0 | Native | 0 | 0 | $0.00 |
| `analyze` | 0 | Native | 0 | 0 | $0.00 |
| `checklist` | 0 | Native | 0 | 0 | $0.00 |
| `specify` | 1 | OpenAI (gpt-5-low) | ~8K | ~3K | $0.09 |
| `plan` | 3 | Gemini+Claude+OpenAI | ~20K | ~8K | $0.34 |
| `tasks` | 1 | OpenAI (gpt-5-low) | ~12K | ~4K | $0.08 |
| `implement` | 2 | OpenAI (codex)+Claude | ~30K | ~10K | $0.11 |
| `validate` | 3 | Gemini+Claude+OpenAI | ~25K | ~8K | $0.34 |
| `audit` | 3 | Gemini Pro+Sonnet+GPT-5 | ~40K | ~12K | $0.78 |
| `unlock` | 3 | Gemini Pro+Sonnet+GPT-5 | ~35K | ~10K | $0.79 |
| `auto` | Strategic | Mixed (all above) | ~170K | ~55K | $2.73 |

### Cost Optimization Strategies

**Minimum Cost** (single cheap agent everywhere):
```toml
[quality_gates]
specify = ["gemini"]
plan = ["gemini"]
tasks = ["gemini"]
implement = ["gemini"]
validate = ["gemini"]
audit = ["gemini"]
unlock = ["gemini"]
# Total: ~$0.50 (vs $2.70)
```

**Balanced** (recommended, current default):
```toml
[quality_gates]
specify = ["code"]                        # $0.10
plan = ["gemini", "claude", "code"]       # $0.35
tasks = ["code"]                          # $0.10
implement = ["gpt_codex", "claude"]       # $0.11
validate = ["gemini", "claude", "code"]   # $0.35
audit = ["gemini-pro", "claude-sonnet", "gpt-5"]  # $0.80
unlock = ["gemini-pro", "claude-sonnet", "gpt-5"] # $0.80
# Total: ~$2.70
```

**Premium** (highest quality):
```toml
[quality_gates]
specify = ["gpt-5"]                       # $0.20
plan = ["gemini-pro", "claude-opus", "gpt-5"]    # $1.20
tasks = ["gpt-5"]                         # $0.20
implement = ["gpt_codex", "claude-opus"]  # $0.35
validate = ["gemini-pro", "claude-opus", "gpt-5"] # $1.20
audit = ["gemini-pro", "claude-opus", "gpt-5"]    # $0.80
unlock = ["gemini-pro", "claude-opus", "gpt-5"]   # $0.80
# Total: ~$4.75
```

---

## Next Steps

- [Pipeline Architecture](pipeline-architecture.md) - State machine and workflow
- [Consensus System](consensus-system.md) - Multi-agent synthesis
- [Quality Gates](quality-gates.md) - Checkpoint configuration
- [Native Operations](native-operations.md) - FREE operations deep dive

---

**File References**:
- Command implementations: `codex-rs/tui/src/chatwidget/spec_kit/commands/`
- Command registry: `codex-rs/tui/src/chatwidget/spec_kit/command_registry.rs`
- Native operations: `codex-rs/tui/src/chatwidget/spec_kit/*_native.rs`
- Auto pipeline: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`
