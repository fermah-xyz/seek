use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OtlpConfig {
    pub endpoint: String,
    pub timeout_secs: u64,
}

impl Default for OtlpConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4317".to_string(),
            timeout_secs: 3,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub export: Option<OtlpConfig>,
    pub filter: Option<String>,
}
