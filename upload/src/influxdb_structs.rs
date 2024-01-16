
use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct InfluxDbConfig {
    pub url: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub file_base_path: String
}
