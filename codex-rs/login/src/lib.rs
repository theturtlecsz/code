mod pkce;
mod server;

// P6-SYNC Phase 5: Device Code Authorization Flow
pub mod device_code;
pub mod device_code_anthropic;
pub mod device_code_google;
pub mod device_code_openai;
pub mod device_code_storage;

pub use server::LoginServer;
pub use server::ServerOptions;
pub use server::ShutdownHandle;
pub use server::run_login_server;

// P6-SYNC Phase 5: Re-export commonly used device code types
pub use device_code::{
    DeviceAuthError, DeviceAuthorizationResponse, DeviceCodeAuth, DeviceCodeProvider, PollError,
    RefreshError, StoredToken, TokenResponse,
};
pub use device_code_anthropic::AnthropicDeviceCode;
pub use device_code_google::GoogleDeviceCode;
pub use device_code_openai::OpenAIDeviceCode;
pub use device_code_storage::{DeviceCodeTokenStorage, TokenStatus};

// Re-export commonly used auth types and helpers from codex-core for compatibility
pub use codex_core::AuthManager;
pub use codex_core::CodexAuth;
pub use codex_core::auth::AuthDotJson;
pub use codex_core::auth::CLIENT_ID;
pub use codex_core::auth::OPENAI_API_KEY_ENV_VAR;
pub use codex_core::auth::get_auth_file;
pub use codex_core::auth::login_with_api_key;
pub use codex_core::auth::logout;
pub use codex_core::auth::try_read_auth_json;
pub use codex_core::auth::write_auth_json;
pub use codex_core::token_data::TokenData;
pub use codex_protocol::mcp_protocol::AuthMode;
