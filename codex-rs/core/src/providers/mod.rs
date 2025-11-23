//! Provider implementations for the multi-provider authentication framework.
//!
//! Each provider module implements the [`ProviderAuth`] trait for its
//! respective service.
//!
//! # Providers
//!
//! - [`openai`]: OpenAI / ChatGPT (refactored from existing auth.rs)
//! - [`anthropic`]: Anthropic / Claude
//! - [`google`]: Google / Gemini

pub mod anthropic;
pub mod google;
pub mod openai;

pub use anthropic::AnthropicAuth;
pub use google::GoogleAuth;
pub use openai::OpenAIAuth;
