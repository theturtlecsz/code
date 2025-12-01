# STAGE0_TASK_BRIEF_TEMPLATE.md

## Purpose

`TASK_BRIEF.md` is the primary output of the Stage0 Dynamic Context Compiler (DCC).

It is:

* The **input context** for NotebookLM Tier 2 ("Divine Truth") alongside `spec.md`.
* A **human-readable briefing** for `/speckit.auto` and downstream agents.
* A compressed, structured view of:

  * relevant memories (from local-memory),
  * code context,
  * docs/issues context,
  * inferred constraints, risks, and assumptions.

It MUST be:

* **Deterministic in structure** (sections + headings).
* **Token-bounded** (respecting `context_compiler.max_tokens` from config).
* **Grounded** (claims traceable back to specific memories/code/docs).

---

## Document Structure

`TASK_BRIEF.md` MUST follow this structure:

```markdown
# Task Brief: {{SPEC_ID}}

## 1. Spec Snapshot

### 1.1 Summary

- <3–7 bullet summary of the spec's intent and scope>

### 1.2 Key Objectives

- <bullet list of concrete goals>
- …

### 1.3 Non-Goals (if identifiable)

- <optional list of things explicitly out of scope>
- …
```

---

```markdown
## 2. Relevant Context (Memories)

### 2.1 High-Priority Memories

For each top memory (e.g. top 5–10 by combined score):

#### Memory {{N}} – {{MEMORY_ID}}

- **Type:** [PATTERN | DECISION | PROBLEM | INSIGHT | OTHER]
- **Score:** {{combined_score}} (sim={{similarity_score}}, dyn={{dynamic_score}})
- **Tags:** {{tag1}}, {{tag2}}, …
- **Summary:** <2–4 sentence summary in your own words>

> Excerpt:
> "{{short excerpt from memory content}}"

### 2.2 Supporting Memories (Optional)

- {{MEMORY_ID}} – {{type}} – {{1-line summary}} (score={{combined_score}})
- …
```

---

````markdown
## 3. Code Context (Optional)

Include when DCC finds relevant `kind="code"` entries.

### 3.1 Key Code Units

For each top code unit:

#### Code Unit {{N}}

- **Location:** `{{repo}}/{{path}}` (symbol: `{{symbol}}`)
- **Role:** <1–2 sentence description of what this code does>
- **Why relevant:** <1–2 sentences linking to the spec>

> Snippet:
> ```rust
> // minimal but representative snippet
> ```
````

### 3.2 Other Code References (Optional)

* `{{repo}}/{{path}}` – `{{symbol}}` – {{1-line note}}
* …

---

```markdown
## 4. Docs & Issues Context (Optional)

### 4.1 Specs / ADRs

- **{{DOC_ID}} – {{title}}**
  - Type: [SPEC | ADR | DOC]
  - Summary: <1–3 sentences>
  - Why relevant: <1–2 sentences>

### 4.2 Issues / PRs

- **{{ISSUE_OR_PR_ID}} – {{title}}**
  - Type: [ISSUE | PR]
  - Status: {{open/closed/merged}}
  - Summary: <1–3 sentences>
  - Why relevant: <1–2 sentences>
```

---

```markdown
## 5. Inferred Constraints & Assumptions

### 5.1 Hard Constraints

- [C1] <constraint>
  - Backed by: `mem-123`, `mem-456`, `code:src/foo.rs#Bar`.

- [C2] …

### 5.2 Working Assumptions

- [A1] <assumption derived from context; clearly marked as assumption>
- [A2] …

(Constraints should reference memory IDs and code/docs when possible.)
```

---

```markdown
## 6. Known Risks & Pitfalls

### 6.1 Risks

- [R1] <risk>
  - Impact: [low | medium | high]
  - Source: <memory/code/doc IDs>

- [R2] …

### 6.2 Historical Pitfalls

- [P1] <anti-pattern / repeated failure>
  - Linked to: `mem-abc`, `mem-def`, `NL_BUG_RETROS_01` section …
```

---

````markdown
## 7. Metadata (Machine-Readable)

```json
{
  "spec_id": "{{SPEC_ID}}",
  "stage0_version": "{{STAGE0_VERSION}}",
  "dcc_config": {
    "max_tokens": {{MAX_TOKENS}},
    "top_k": {{TOP_K}},
    "similarity_weight": {{SIM_W}},
    "dynamic_score_weight": {{DYN_W}},
    "diversity_lambda": {{MMR_LAMBDA}}
  },
  "memories_used": [
    {
      "id": "mem-123",
      "similarity": 0.91,
      "dynamic_score": 0.82,
      "combined_score": 0.88
    }
  ],
  "code_refs": [
    {
      "repo": "codex-rs",
      "path": "src/foo.rs",
      "symbol": "FooBar"
    }
  ]
}
```
````

---

## Constraints

- Respect `context_compiler.max_tokens` (drop the lowest-value entries if needed).
- Never hallucinate memory IDs; only use IDs actually returned by DCC/local-memory.
- Prefer concise summaries + short excerpts over full dumps of content.
