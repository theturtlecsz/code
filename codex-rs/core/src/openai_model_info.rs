use crate::model_family::ModelFamily;

/// Metadata about a model, particularly OpenAI models.
/// We may want to consider including details like the pricing for
/// input tokens, output tokens, etc., though users will need to be able to
/// override this in config.toml, as this information can get out of date.
/// Though this would help present more accurate pricing information in the UI.
#[derive(Debug)]
pub(crate) struct ModelInfo {
    /// Size of the context window in tokens. This is the maximum size of the input context.
    pub(crate) context_window: u64,

    /// Maximum number of output tokens that can be generated for the model.
    pub(crate) max_output_tokens: u64,

    /// Token threshold where we should automatically compact conversation history. This considers
    /// input tokens + output tokens of this turn.
    pub(crate) auto_compact_token_limit: Option<i64>,
}

impl ModelInfo {
    const fn new(context_window: u64, max_output_tokens: u64) -> Self {
        Self {
            context_window,
            max_output_tokens,
            auto_compact_token_limit: None,
        }
    }
}

pub(crate) fn get_model_info(model_family: &ModelFamily) -> Option<ModelInfo> {
    let slug = model_family.slug.as_str();
    match slug {
        // OSS models have a 128k shared token pool.
        // Arbitrarily splitting it: 3/4 input context, 1/4 output.
        // https://openai.com/index/gpt-oss-model-card/
        "gpt-oss-20b" => Some(ModelInfo::new(96_000, 32_000)),
        "gpt-oss-120b" => Some(ModelInfo::new(96_000, 32_000)),
        // https://platform.openai.com/docs/models/o3
        "o3" => Some(ModelInfo::new(200_000, 100_000)),

        // https://platform.openai.com/docs/models/o4-mini
        "o4-mini" => Some(ModelInfo::new(200_000, 100_000)),

        // https://platform.openai.com/docs/models/codex-mini-latest
        "codex-mini-latest" => Some(ModelInfo::new(200_000, 100_000)),

        // As of Jun 25, 2025, gpt-4.1 defaults to gpt-4.1-2025-04-14.
        // https://platform.openai.com/docs/models/gpt-4.1
        "gpt-4.1" | "gpt-4.1-2025-04-14" => Some(ModelInfo::new(1_047_576, 32_768)),

        // As of Jun 25, 2025, gpt-4o defaults to gpt-4o-2024-08-06.
        // https://platform.openai.com/docs/models/gpt-4o
        "gpt-4o" | "gpt-4o-2024-08-06" => Some(ModelInfo::new(128_000, 16_384)),

        // https://platform.openai.com/docs/models/gpt-4o?snapshot=gpt-4o-2024-05-13
        "gpt-4o-2024-05-13" => Some(ModelInfo::new(128_000, 4_096)),

        // https://platform.openai.com/docs/models/gpt-4o?snapshot=gpt-4o-2024-11-20
        "gpt-4o-2024-11-20" => Some(ModelInfo::new(128_000, 16_384)),

        // https://platform.openai.com/docs/models/gpt-3.5-turbo
        "gpt-3.5-turbo" => Some(ModelInfo::new(16_385, 4_096)),

        // GPT-5 codex variants (any gpt-5* with "codex" gets auto-compact optimization)
        _ if slug.starts_with("gpt-5") && slug.contains("codex") => Some(ModelInfo {
            context_window: 272_000,
            max_output_tokens: 128_000,
            auto_compact_token_limit: Some(350_000),
        }),

        // GPT-5 base and adaptive reasoning models
        _ if slug.starts_with("gpt-5") => Some(ModelInfo::new(272_000, 128_000)),

        // Standalone codex models
        _ if slug.starts_with("codex-") => Some(ModelInfo::new(272_000, 128_000)),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_family::find_family_for_model;

    #[test]
    fn test_gpt5_base_model() {
        let family = find_family_for_model("gpt-5").expect("gpt-5 should have family");
        let info = get_model_info(&family).expect("gpt-5 should be recognized");

        assert_eq!(info.context_window, 272_000, "GPT-5 context window");
        assert_eq!(info.max_output_tokens, 128_000, "GPT-5 max output");
        assert_eq!(
            info.auto_compact_token_limit, None,
            "GPT-5 base has no auto-compact"
        );
    }

    #[test]
    fn test_gpt5_1_model() {
        let family = find_family_for_model("gpt-5.1").expect("gpt-5.1 should have family");
        let info = get_model_info(&family).expect("gpt-5.1 should be recognized");

        assert_eq!(info.context_window, 272_000, "GPT-5.1 context window");
        assert_eq!(info.max_output_tokens, 128_000, "GPT-5.1 max output");
        assert_eq!(
            info.auto_compact_token_limit, None,
            "GPT-5.1 has no auto-compact"
        );
    }

    #[test]
    fn test_gpt5_codex_model() {
        let family = find_family_for_model("gpt-5-codex").expect("gpt-5-codex should have family");
        let info = get_model_info(&family).expect("gpt-5-codex should be recognized");

        assert_eq!(info.context_window, 272_000, "GPT-5-codex context window");
        assert_eq!(info.max_output_tokens, 128_000, "GPT-5-codex max output");
        assert_eq!(
            info.auto_compact_token_limit,
            Some(350_000),
            "GPT-5-codex should have auto-compact for agentic workflows"
        );
    }

    #[test]
    fn test_gpt5_1_codex_model() {
        let family =
            find_family_for_model("gpt-5.1-codex").expect("gpt-5.1-codex should have family");
        let info = get_model_info(&family).expect("gpt-5.1-codex should be recognized");

        assert_eq!(info.context_window, 272_000, "GPT-5.1-codex context window");
        assert_eq!(info.max_output_tokens, 128_000, "GPT-5.1-codex max output");
        assert_eq!(
            info.auto_compact_token_limit,
            Some(350_000),
            "GPT-5.1-codex should have auto-compact (contains 'codex')"
        );
    }

    #[test]
    fn test_gpt5_1_codex_mini_model() {
        let family = find_family_for_model("gpt-5.1-codex-mini")
            .expect("gpt-5.1-codex-mini should have family");
        let info = get_model_info(&family).expect("gpt-5.1-codex-mini should be recognized");

        assert_eq!(
            info.context_window, 272_000,
            "GPT-5.1-codex-mini context window"
        );
        assert_eq!(
            info.max_output_tokens, 128_000,
            "GPT-5.1-codex-mini max output"
        );
        assert_eq!(
            info.auto_compact_token_limit,
            Some(350_000),
            "GPT-5.1-codex-mini should have auto-compact (contains 'codex')"
        );
    }

    #[test]
    fn test_generic_codex_model() {
        let family =
            find_family_for_model("codex-latest").expect("codex-latest should have family");
        let info = get_model_info(&family).expect("codex-latest should be recognized");

        assert_eq!(info.context_window, 272_000, "codex-* context window");
        assert_eq!(info.max_output_tokens, 128_000, "codex-* max output");
    }

    #[test]
    fn test_unknown_model_returns_none() {
        // Unknown slug won't have family, so get_model_info gets None as input
        let family = find_family_for_model("unknown-model-xyz");
        assert!(family.is_none(), "Unknown models should have no family");
    }
}
