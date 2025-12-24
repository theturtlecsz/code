use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, TS, schemars::JsonSchema)]
pub struct CustomPrompt {
    pub name: String,
    pub path: PathBuf,
    pub content: String,
}
