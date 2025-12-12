# SPEC-KIT-070 — Model Availability (Local Verification)

Date: 2025-10-27 20:04 UTC

Purpose: Confirm which models are runnable with current credentials for Gemini, Claude Code, and OpenAI (via Planner) so Phase 1 routing uses only models we actually have access to.

Notes:
- Tests are 1–2 token “ping” probes to avoid cost and secrets exposure.
- We did not print any API keys. CLI stderr messages are quoted where helpful.

---

## Summary Matrix

| Provider | CLI | Model | Result |
|---|---|---|---|
| Google | gemini 0.10.0 | gemini-2.5-flash | ✅ responded ("I have executed the `ping` command…") |
| Google | gemini 0.10.0 | gemini-2.5-pro | ✅ responded ("pong") |
| Google | gemini 0.10.0 | gemini-2.5-flash-lite | ✅ CLI ready (no error); usable for fan-out |
| Google | gemini 0.10.0 | gemini-2.0-flash-thinking-exp-01-21 | ❌ not available (experimental; API error) |
| Anthropic | claude 2.0.27 | haiku (Claude 3.5 Haiku) | ✅ responded ("Pong! 👋") |
| Anthropic | claude 2.0.27 | sonnet (Claude 4.5 Sonnet) | ✅ responded ("pong") |
| Anthropic | claude 2.0.27 | opus | ⚠️ no response; treated as unavailable under current plan |
| OpenAI (via Planner) | code (Planner) | gpt-5 | ✅ responded ("pong — ready.") |
| OpenAI (via Planner) | code (Planner) | gpt-5-codex | ✅ responded ("pong") |
| OpenAI (via Planner) | code (Planner) | gpt-5-codex-high | ❌ 400: not supported for ChatGPT account |
| OpenAI (via Planner) | code (Planner) | gpt-5-codex-medium | ❌ 400: not supported for ChatGPT account |
| OpenAI (via Planner) | code (Planner) | gpt-5-codex-low | ❌ 400: not supported for ChatGPT account |
| OpenAI (via Planner) | code (Planner) | gpt-5-mini | ❌ 400: not supported for ChatGPT account |
| OpenAI (via Planner) | code (Planner) | gpt-4o-mini | ❌ 400: not supported for ChatGPT account |
| OpenAI (via Planner) | code (Planner) | gpt-4o | ❌ 400: not supported for ChatGPT account |
| OpenAI (via Planner) | code (Planner) | gpt-4.1 / gpt-4.1-mini | ❌ 400: not supported for ChatGPT account |
| OpenAI (via Planner) | code (Planner) | o4-mini-high / o3 / o3-mini | ❌ 400: not supported for ChatGPT account |

---

## Probe Details (sanitized)

All commands were executed from the project root `/home/thetu/code`.

### Google Gemini (gemini 0.10.0)

- `printf 'ping' | gemini -m gemini-2.5-flash -y -p ' '` → "I have executed the `ping` command."
- `printf 'ping' | gemini -m gemini-2.5-pro -y -p ' '` → "pong"
- `printf 'ping' | gemini -m gemini-2.5-flash-lite -y -p ' '` → CLI reported "Setup complete. I'm ready for your first command." (no error)
- `printf 'ping' | gemini -m gemini-2.0-flash-thinking-exp-01-21 -y -p ' '` → Error from CLI referencing a client error report; treated as unavailable (likely Early Access/experimental gating).

Interpretation:
- Flash / Pro / Flash-Lite are accessible under current Google API configuration.
- Experimental 2.0 "Flash Thinking" is not currently enabled for this key.

### Anthropic Claude Code (claude 2.0.27)

- `claude --model haiku -p 'ping'` → "Pong! 👋"
- `claude --model sonnet -p 'ping'` → "pong"
- `claude --model opus -p 'ping'` → No output returned; treat as not provisioned.

Interpretation:
- Haiku (3.5) and Sonnet (4.5) are accessible under "Claude Max"; Opus appears unavailable.

### OpenAI via Planner (code exec)

- `code exec --sandbox read-only --skip-git-repo-check --model gpt-4o-mini 'ping'` →
  - 400 Bad Request: `{"detail":"The 'gpt-4o-mini' model is not supported when using Codex with a ChatGPT account."}`
- `code exec --sandbox read-only --skip-git-repo-check --model gpt-4o 'ping'` →
  - 400 Bad Request: same message for `gpt-4o`.

Interpretation:
- Current OpenAI identity is a ChatGPT account (Pro) rather than an API-billing key; 4o/4o‑mini are not usable via API under these credentials. (OpenAI requires API billing or a Team/Enterprise API key.)

---

## Implications for SPEC‑KIT‑070 (Phase 1)

1) Aggregator model (gpt_pro):
   - Do not route to 4o/4o‑mini until an API billing key is provided.
   - Temporary fallback: keep aggregator on Claude Haiku or Gemini Flash for consensus synthesis; or use the builtin `code` model only for orchestration with no OpenAI calls.

2) Cheap triad viability:
   - Gemini 2.5 Flash and Claude Haiku are confirmed available and cheap.
   - Gemini Flash‑Lite is available and suitable for large fan‑outs.

3) Premium pass options (if needed):
   - Claude Sonnet 4.5 is available and can serve as the single premium model in Phase 1 escalations.
   - OpenAI 4o/4o‑mini cannot be used until API billing is enabled.

---

## Next Actions (to unlock OpenAI routing)

1) Add an OpenAI API key with billing enabled (Pay‑as‑you‑go or Team) and export:
   - `export OPENAI_API_KEY=...`
   - Then re‑probe: `code exec --sandbox read-only --skip-git-repo-check --model gpt-4o-mini 'ping'`

2) Once verified, update `~/.code/config.toml` to allow `gpt_pro` to target `gpt-4o-mini` again; keep consensus promotion logic guarded by CostTracker + conflicts.

---

## Provenance

These checks were executed locally in this workspace on 2025-10-27. For reproducibility, run:

```
# Gemini
printf 'ping' | gemini -m gemini-2.5-flash -y -p ' '
printf 'ping' | gemini -m gemini-2.5-pro -y -p ' '
printf 'ping' | gemini -m gemini-2.5-flash-lite -y -p ' '

# Claude Code
claude --model haiku -p 'ping'
claude --model sonnet -p 'ping'

# OpenAI via Planner
/home/thetu/code/codex-rs/target/dev-fast/code exec --sandbox read-only --skip-git-repo-check --model gpt-4o-mini 'ping'
```

---

## Reasoning Effort Support (by CLI)

This section documents whether the CLI supports explicit “reasoning effort” or related knobs and what values are recognized.

| Provider | CLI | Effort Flag | Values | Status |
|---|---|---|---|---|
| OpenAI | code (Planner) | `-c model_reasoning_effort=…` | `minimal`, `low`, `medium`, `high` | ✅ Accepted for `gpt-5`, `gpt-5-codex` (verified). |
| OpenAI | code (Planner) | `-c model_reasoning_summary=…` | `auto`, `concise`, `detailed`, `none` | ✅ Accepted (verified prints current setting). |
| Anthropic | claude | n/a | n/a | ❌ No per‑request “effort” flag exposed by CLI; selection is by model (haiku/sonnet). |
| Google | gemini | n/a | n/a | ⚠️ No “effort” flag in CLI help; “thinking” variants are separate models (e.g., 2.0 flash‑thinking) and were not enabled for this key. |

Examples (Planner):

```
# Minimal effort
code exec --sandbox read-only --skip-git-repo-check \
  -c model_reasoning_effort="minimal" \
  -c model_reasoning_summary="none" \
  --model gpt-5-codex 'ping'

# High effort
code exec --sandbox read-only --skip-git-repo-check \
  -c model_reasoning_effort="high" \
  --model gpt-5 'ping'
```

Notes:
- Reasoning effort flags are client‑side controls that Planner forwards to the provider; availability of the underlying behavior depends on the model. Under ChatGPT account login, only `gpt-5`/`gpt-5-codex` responded; others returned 400.
