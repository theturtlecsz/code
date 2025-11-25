use codex_core::protocol_config_types::ReasoningEffort;
use codex_protocol::mcp_protocol::AuthMode;

/// A simple preset pairing a model slug with a reasoning effort.
#[derive(Debug, Clone, Copy)]
pub struct ModelPreset {
    /// Stable identifier for the preset.
    pub id: &'static str,
    /// Display label shown in UIs.
    pub label: &'static str,
    /// Short human description shown next to the label in UIs.
    pub description: &'static str,
    /// Model slug (e.g., "gpt-5").
    pub model: &'static str,
    /// Reasoning effort to apply for this preset.
    pub effort: Option<ReasoningEffort>,
}

const PRESETS: &[ModelPreset] = &[
    // ═══════════════════════════════════════════════════════════════
    // Gemini Family (Google)
    // ═══════════════════════════════════════════════════════════════
    ModelPreset {
        id: "gemini-3-pro",
        label: "Gemini 3 Pro",
        description: "— #1 LMArena (1501 Elo), PhD-level reasoning, best for coding/math ($2/$12)",
        model: "gemini-3-pro",
        effort: None,
    },
    ModelPreset {
        id: "gemini-2.5-pro",
        label: "Gemini 2.5 Pro",
        description: "— strong reasoning and multimodal, cost-effective premium tier ($1.25/$10)",
        model: "gemini-2.5-pro",
        effort: None,
    },
    ModelPreset {
        id: "gemini-2.5-flash",
        label: "Gemini 2.5 Flash",
        description: "— fastest and cheapest, great for quick tasks and prototyping ($0.30/$2.50)",
        model: "gemini-2.5-flash",
        effort: None,
    },
    // ═══════════════════════════════════════════════════════════════
    // Claude Family (Anthropic)
    // ═══════════════════════════════════════════════════════════════
    ModelPreset {
        id: "claude-opus-4.5",
        label: "Claude Opus 4.5",
        description: "— most capable Claude, best for complex/creative tasks, ultra premium ($15/$75)",
        model: "claude-opus-4.5",
        effort: None,
    },
    ModelPreset {
        id: "claude-sonnet-4.5",
        label: "Claude Sonnet 4.5",
        description: "— balanced performance and cost, excellent for agents and coding ($3/$15)",
        model: "claude-sonnet-4.5",
        effort: None,
    },
    ModelPreset {
        id: "claude-haiku-4.5",
        label: "Claude Haiku 4.5",
        description: "— fast and cost-efficient, good for simpler tasks ($1/$5)",
        model: "claude-haiku-4.5",
        effort: None,
    },
    // ═══════════════════════════════════════════════════════════════
    // GPT-5.1 Family (OpenAI) - with reasoning levels
    // ═══════════════════════════════════════════════════════════════

    // GPT-5.1 Standard (with reasoning variants)
    ModelPreset {
        id: "gpt-5.1-minimal",
        label: "GPT-5.1 Minimal",
        description: "— fastest responses with limited reasoning; ideal for coding, instructions ($1.25/$10)",
        model: "gpt-5",
        effort: Some(ReasoningEffort::Minimal),
    },
    ModelPreset {
        id: "gpt-5.1-low",
        label: "GPT-5.1 Low",
        description: "— balances speed with some reasoning; straightforward queries ($1.25/$10)",
        model: "gpt-5",
        effort: Some(ReasoningEffort::Low),
    },
    ModelPreset {
        id: "gpt-5.1-medium",
        label: "GPT-5.1 Medium",
        description: "— default setting; solid balance of reasoning depth and latency ($1.25/$10)",
        model: "gpt-5",
        effort: Some(ReasoningEffort::Medium),
    },
    ModelPreset {
        id: "gpt-5.1-high",
        label: "GPT-5.1 High",
        description: "— maximizes reasoning depth for complex or ambiguous problems ($1.25/$10)",
        model: "gpt-5",
        effort: Some(ReasoningEffort::High),
    },
    // GPT-5.1 Codex (code specialist with reasoning variants)
    ModelPreset {
        id: "gpt-5.1-codex-low",
        label: "GPT-5.1 Codex Low",
        description: "— code specialist with light reasoning ($1.25/$10)",
        model: "gpt-5-codex",
        effort: Some(ReasoningEffort::Low),
    },
    ModelPreset {
        id: "gpt-5.1-codex-medium",
        label: "GPT-5.1 Codex Medium",
        description: "— code specialist with balanced reasoning ($1.25/$10)",
        model: "gpt-5-codex",
        effort: None,
    },
    ModelPreset {
        id: "gpt-5.1-codex-high",
        label: "GPT-5.1 Codex High",
        description: "— code specialist with deep reasoning ($1.25/$10)",
        model: "gpt-5-codex",
        effort: Some(ReasoningEffort::High),
    },
];

pub fn builtin_model_presets(_auth_mode: Option<AuthMode>) -> Vec<ModelPreset> {
    PRESETS.to_vec()
}
