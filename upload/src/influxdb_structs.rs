
use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct InfluxDbConfig {
    pub url: String,
    pub org: String,
    pub bucket: String,
    pub token: String,
    pub file_base_path: String
}
