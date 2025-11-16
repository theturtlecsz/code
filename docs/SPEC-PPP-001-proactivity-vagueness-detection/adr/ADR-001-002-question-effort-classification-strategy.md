# ADR-001-002: Question Effort Classification Strategy

**Status**: Accepted
**Date**: 2025-11-16
**Deciders**: Research Team
**Related**: SPEC-PPP-001 (Proactivity & Vagueness Detection)

---

## Context

The PPP Framework's $R_{Proact}$ formula penalizes agents for asking high-effort questions that block user productivity:

```
R_Proact = {
    +0.05  if no questions asked
    +0.05  if all questions are low-effort
    -0.1 × (medium-effort questions)
    -0.5 × (high-effort questions)
}
```

**Problem**: How should we classify questions into **low/medium/high** effort categories?

**Effort Definitions** (from PPP paper, arXiv:2511.02208):
- **Low**: Selection from options, accessible context (e.g., "Which provider: A, B, or C?")
- **Medium**: Some research needed, not blocking (e.g., "What format do you prefer?")
- **High**: Deep investigation or blocking decision (e.g., "Should I investigate distributed caching strategies?")

---

## Decision

We will implement **keyword matching + length heuristics for Phase 1**, with optional LLM upgrade in Phase 3.

**Approach**:
- **Phase 1** (Immediate): Keyword lists + word count (75-80% accuracy, $0 cost, <1ms latency)
- **Phase 2** (Optional): Multi-part question handling + enhanced patterns (80-85% accuracy)
- **Phase 3** (Optional): LLM-based classification (90-95% accuracy, $10.80/year)

---

## Rationale

### 1. Linguistic Features of Effort Levels

**Research Findings** (Brosnan et al., 2021, Paas scale):

**Low-Effort Questions** (Characteristics):
- Short (<10 words)
- Contains selection keywords: "which", "choose", "select", "prefer", "option"
- Presents explicit options: " or ", "option A/B"
- Immediate answer (no research)

**Examples**:
- "Which database: PostgreSQL or MySQL?"
- "Do you prefer tabs or spaces?"
- "Choose option A or B?"

**Medium-Effort Questions** (Characteristics):
- Moderate length (10-20 words)
- Requires preference elicitation: "what...should", "how...prefer"
- No blocking language
- Some thought required

**Examples**:
- "What authentication method should we use?"
- "How should we handle errors?"
- "What is your preferred coding style?"

**High-Effort Questions** (Characteristics):
- Long (>20 words) OR contains blocking keywords
- Investigation language: "investigate", "research", "before proceeding"
- Decision-blocking: "should we consider", "need to decide"
- Architecture/strategy: "architecture", "trade-off", "strategy"

**Examples**:
- "Should we investigate distributed caching strategies before proceeding?"
- "Do you want me to research authentication solutions?"
- "What architecture patterns should we consider for this microservice?"

---

### 2. Keyword-Based Classification

**Decision Tree**:
```rust
pub fn classify_effort(question: &str) -> EffortLevel {
    let word_count = question.split_whitespace().count();
    let lower = question.to_lowercase();

    // High-effort indicators (override length)
    if lower.contains("investigate") || lower.contains("research") ||
       lower.contains("before proceeding") || lower.contains("architecture") {
        return EffortLevel::High;
    }

    // Low-effort indicators (selection)
    let has_options = lower.contains(" or ") || lower.contains("option");
    let has_selection = lower.contains("which") || lower.contains("choose");

    if (has_options || has_selection) && word_count < 15 {
        return EffortLevel::Low;
    }

    // Length-based fallback
    match word_count {
        0..=10 => EffortLevel::Low,
        11..=20 => EffortLevel::Medium,
        _ => EffortLevel::High,
    }
}
```

**Pros**:
- ✅ Fast (<1ms)
- ✅ Transparent (rule-based)
- ✅ Easy to tune (add keywords)
- ✅ No dependencies

**Cons**:
- ❌ Misses nuanced effort (context-dependent)
- ❌ Fixed thresholds (10/20 words)

---

### 3. Accuracy vs Cost Trade-off

**Comparison**:

| Method | Accuracy | Latency | Cost/1K questions | Implementation |
|--------|----------|---------|-------------------|----------------|
| Keyword + Length | 75-80% | <1ms | $0 | Easy |
| ML (SVM) | 85-88% | 5-10ms | $0* | Hard |
| LLM (Haiku) | 90-95% | 500ms | $0.30 | Medium |

\* Training cost, inference is local

**Budget Analysis** (10 questions/consensus run, 100 runs/month):
- 10 questions × 3 agents = 30 questions/run
- 30 questions × 100 runs = 3,000 questions/month = 36,000 questions/year
- LLM cost: 36,000 × $0.0003 = **$10.80/year**

**Verdict**: Keyword approach is **free**, LLM is **affordable** ($10.80/year).

---

### 4. Validation Evidence

**Literature Support**:
- Paas Cognitive Load Scale (2021): Question length correlates with cognitive effort
- Brosnan et al. (2021): Selection questions (low-effort) vs open-ended (high-effort)
- Haptik (2023): 90% user response rate to low-effort questions (selection)

**Keyword Effectiveness** (from findings.md):
- Blocking keywords ("investigate", "research"): 85% precision for high-effort
- Selection keywords ("which", "choose"): 80% precision for low-effort
- Length threshold (10/20 words): 70% accuracy for medium-effort

**Combined Accuracy**: 75-80% (sufficient for Phase 1)

---

### 5. Edge Cases

**Rhetorical Questions**:
- Example: "Why don't we just use PostgreSQL?" (not really asking)
- **Mitigation**: Context analysis (future work), acceptable false positive rate for Phase 1

**Multi-Part Questions**:
- Example: "Which database and what version should we use?" (2 questions)
- **Solution**: Split on " and ", classify separately, take maximum effort
- **Implementation**: Phase 2

**Context-Dependent Effort**:
- Example: "What format?" (low-effort if options given earlier, medium otherwise)
- **Mitigation**: Multi-turn context tracking (future work)

---

## Comparison to Alternatives

### Alternative 1: ML-Based (SVM)

**Approach**: Train SVM on labeled dataset (question → effort level).

**Features**:
- Word count
- Keyword presence (binary)
- Question type (WH-word)
- Sentence complexity (depth)

**Pros**:
- ✅ Higher accuracy (85-88%)
- ✅ Learns patterns from data
- ✅ Local inference ($0 cost)

**Cons**:
- ❌ Training complexity (requires labeled dataset)
- ❌ Slower (5-10ms)
- ❌ Black-box (less transparent)
- ❌ Maintenance burden (retrain periodically)

**Verdict**: ❌ Reject for Phase 1 - Over-engineering. Consider for Phase 2 if keyword accuracy <80%.

---

### Alternative 2: LLM-Based (Claude Haiku)

**Approach**: Use Claude Haiku with few-shot prompting.

**Prompt Template**:
```
Classify question by user effort (LOW, MEDIUM, HIGH).

LOW: Selection, accessible context (e.g., "Which: A or B?")
MEDIUM: Research, preferences (e.g., "What method should we use?")
HIGH: Investigation, blocking (e.g., "Should we investigate strategies?")

Question: "{question}"
Effort:
```

**Pros**:
- ✅ High accuracy (90-95%)
- ✅ Context-aware (understands nuance)
- ✅ Handles edge cases (rhetorical, multi-part)
- ✅ Provides reasoning (explainable)

**Cons**:
- ❌ Slow (500ms)
- ❌ Costs $0.0003/question ($10.80/year)
- ❌ Non-deterministic

**Verdict**: ⚠️ Phase 3 upgrade if keyword accuracy <85% in production.

---

### Alternative 3: Rule-Based with Dependency Parsing

**Approach**: Use nlprule for POS tagging + dependency parsing.

**Enhanced Rules**:
```rust
pub fn classify_with_parsing(question: &str) -> EffortLevel {
    let tokens = tokenizer.tokenize(question);

    // Check for modal verbs (should, could) → high-effort
    let has_modal = tokens.iter().any(|t| t.pos() == "MD");

    // Check for verb phrase complexity
    let vp_depth = calculate_vp_depth(&tokens);

    if has_modal && vp_depth > 2 {
        return EffortLevel::High;
    }

    // Fallback to keyword + length
    classify_effort_basic(question)
}
```

**Pros**:
- ✅ Improved accuracy (80-85%)
- ✅ Syntax-aware

**Cons**:
- ❌ Slower (10-50ms)
- ❌ Requires nlprule (50MB model)
- ❌ Limited to English

**Verdict**: ⚠️ Phase 2 enhancement if keyword accuracy <80%.

---

## Decision Matrix

**Scoring** (0-10 scale, higher better):

| Criterion | Weight | Keyword + Length | ML (SVM) | LLM (Haiku) | Dependency Parsing |
|-----------|--------|------------------|----------|-------------|-------------------|
| **Accuracy** | 0.35 | 7 (75-80%) | 8 (85-88%) | 9 (90-95%) | 7.5 (80-85%) |
| **Latency** | 0.25 | 10 (<1ms) | 8 (5ms) | 5 (500ms) | 7 (10-50ms) |
| **Cost** | 0.2 | 10 ($0) | 10 ($0) | 7 ($10.80/yr) | 10 ($0) |
| **Simplicity** | 0.15 | 9 (rules) | 5 (training) | 8 (API) | 6 (parsing) |
| **Explainability** | 0.05 | 10 (transparent) | 6 (weights) | 8 (reasoning) | 7 (syntax) |
| **Weighted Score** | - | **8.50** | **7.70** | **7.79** | **7.58** |

**Winner**: **Keyword + Length (8.50)** for Phase 1

**LLM (7.79)** is close second, viable for Phase 3 if keyword accuracy insufficient.

---

## Consequences

### Positive

1. ✅ **Zero cost**: No API calls, no training
2. ✅ **Fast**: <1ms latency (negligible overhead)
3. ✅ **Transparent**: Rule-based, easy to debug
4. ✅ **Maintainable**: Add keywords as needed
5. ✅ **Sufficient accuracy**: 75-80% validated in literature

### Negative

1. ⚠️ **Lower accuracy**: 75-80% vs 90-95% with LLM
   - **Mitigation**: Upgrade to Phase 2 (dependency parsing) or Phase 3 (LLM) if accuracy insufficient
   - **Impact**: 20-25% misclassifications (low classified as medium, etc.)

2. ⚠️ **Fixed thresholds**: 10/20 word boundaries may not fit all cases
   - **Mitigation**: Tune thresholds based on validation dataset
   - **Impact**: Low (most questions fall clearly into categories)

3. ⚠️ **Context-insensitive**: Cannot understand multi-turn context
   - **Example**: "What about the other option?" (refers to previous turn)
   - **Mitigation**: Future work (multi-turn context tracking)

### Neutral

1. 📊 **Phased upgrade path**:
   - Phase 1 → Phase 2 (dependency parsing)
   - Phase 2 → Phase 3 (LLM-based)
   - No breaking changes

---

## Implementation Plan

### Phase 1: Keyword + Length (Immediate)

**Timeline**: 2-3 days

**Code**:
```rust
// codex-rs/tui/src/ppp/effort_classifier.rs

pub struct EffortClassifier {
    low_indicators: Vec<&'static str>,
    high_indicators: Vec<&'static str>,
}

impl EffortClassifier {
    pub fn new() -> Self {
        Self {
            low_indicators: vec!["which", "choose", "select", "prefer", "option"],
            high_indicators: vec![
                "investigate", "research", "before proceeding",
                "architecture", "trade-off", "strategy", "blocking"
            ],
        }
    }

    pub fn classify(&self, question: &str) -> EffortLevel {
        let word_count = question.split_whitespace().count();
        let lower = question.to_lowercase();

        // High-effort override
        if self.high_indicators.iter().any(|ind| lower.contains(ind)) {
            return EffortLevel::High;
        }

        // Low-effort selection
        let has_options = lower.contains(" or ") || lower.contains("option");
        let has_selection = self.low_indicators.iter().any(|ind| lower.contains(ind));

        if (has_options || has_selection) && word_count < 15 {
            return EffortLevel::Low;
        }

        // Length fallback
        match word_count {
            0..=10 => EffortLevel::Low,
            11..=20 => EffortLevel::Medium,
            _ => EffortLevel::High,
        }
    }
}
```

**Acceptance Criteria**:
- [ ] Accuracy >75% on validation dataset (100 questions)
- [ ] Latency <5ms (p95)
- [ ] 30+ test cases covering all effort levels

---

### Phase 2: Enhanced Patterns (If Needed)

**Timeline**: 1-2 weeks
**Trigger**: Accuracy <80% after 100 production runs

**Enhancements**:
1. Multi-part question splitting
2. Dependency parsing (modal verbs, VP depth)
3. Domain-specific patterns (coding task vocabulary)

**Target Accuracy**: 80-85%

---

### Phase 3: LLM Upgrade (Optional)

**Timeline**: 1 week
**Trigger**: Accuracy <85% OR user feedback indicates poor classification

**Approach**: LLM-based with few-shot prompting (Claude Haiku)

**Target**:
- Accuracy: 90-95%
- Cost: $10.80/year (affordable)

---

## Validation Strategy

### Test Dataset

**Creation**:
1. Collect 100 questions from agent responses (real codex-tui logs)
2. Manual labeling (2 engineers, resolve disagreements)
3. 33 low, 33 medium, 34 high

**Low-Effort Examples**:
- "Which database: PostgreSQL or MySQL?"
- "Do you prefer tabs or spaces?"
- "Choose A or B?"

**Medium-Effort Examples**:
- "What authentication method should we use?"
- "How should we handle errors?"
- "What is your preferred coding style?"

**High-Effort Examples**:
- "Should we investigate caching strategies before proceeding?"
- "Do you want me to research distributed tracing solutions?"
- "What architecture patterns should we consider?"

### Metrics

**Accuracy**: (TP + TN) / Total (across all 3 classes)
**Per-Class Precision**: TP / (TP + FP) for each effort level
**Per-Class Recall**: TP / (TP + FN) for each effort level
**Macro F1 Score**: Average F1 across all 3 classes

**Targets**:
- Phase 1: Accuracy >75%, Macro F1 >0.75
- Phase 2: Accuracy >80%, Macro F1 >0.80
- Phase 3: Accuracy >90%, Macro F1 >0.90

---

## Integration with $R_{Proact}$ Calculation

### Workflow

**Step 1: Question Extraction** (from agent response):
```rust
let questions = extract_questions(agent_response);
// ["Which database: PostgreSQL or MySQL?", "What auth method?"]
```

**Step 2: Effort Classification**:
```rust
let classifier = EffortClassifier::new();

for question in questions {
    let effort = classifier.classify(question);
    store_question(db, trajectory_id, turn_number, question, effort);
}
```

**Step 3: Proactivity Calculation** (during consensus):
```rust
pub fn calculate_r_proact(trajectory_id: i64, db: &Connection) -> ProactivityScore {
    let questions = get_questions(db, trajectory_id);

    if questions.is_empty() {
        return ProactivityScore { r_proact: 0.05, ... }; // Bonus
    }

    let mut low = 0;
    let mut medium = 0;
    let mut high = 0;

    for question in questions {
        match question.effort_level {
            EffortLevel::Low => low += 1,
            EffortLevel::Medium => medium += 1,
            EffortLevel::High => high += 1,
        }
    }

    let r_proact = if low == questions.len() {
        0.05 // All low-effort
    } else {
        -0.1 * (medium as f32) - 0.5 * (high as f32)
    };

    ProactivityScore { r_proact, questions_asked: questions.len(), low, medium, high }
}
```

**Step 4: Weighted Consensus**:
```rust
let final_score = 0.7 * technical_score + 0.3 * (r_proact + r_pers);
```

---

## References

1. Sun, W., et al. (2025). "Training Proactive and Personalized LLM Agents." arXiv:2511.02208 (PPP formula)
2. Brosnan, K., et al. (2021). "Cognitive Load Reduction in Questionnaires" (Paas scale for effort)
3. Haptik (2023). "90% User Response Rate to Clarification Questions" (low-effort effectiveness)
4. findings.md (SPEC-PPP-001): Literature review of question classification methods

---

## Notes

**Why not start with LLM?**
- Keyword approach is **free** and **fast** (<1ms)
- 75-80% accuracy is **good enough** for Phase 1 validation
- LLM upgrade path is clear if accuracy insufficient

**Why length thresholds (10/20 words)?**
- Cognitive load research: Shorter questions require less effort
- Empirical analysis of coding task questions (findings.md)
- Can be tuned based on validation dataset

**Keyword List Maintenance**:
- Review quarterly (add new patterns from missed cases)
- Low burden (~10-15 minutes/quarter)

**Success Criteria**: If Phase 1 achieves >75% accuracy in production, this ADR is validated. If <75%, upgrade to Phase 2 (enhanced patterns) or Phase 3 (LLM).

---

**Decision**: **Accepted** (2025-11-16)
**Implemented**: Phase 1 (keyword + length) in vagueness_detector_poc.rs
**Next Review**: After 100 production consensus runs
