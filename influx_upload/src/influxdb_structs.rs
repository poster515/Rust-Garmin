use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct InfluxDbConfig {
    pub url: String,
    pub org: String,
    pub bucket: String,
    pub token: String,
    pub file_base_path: String,
    pub upload_json_files: bool,
    pub upload_fit_files: bool,
    pub records_to_include: Value,
    pub files_to_prune: Value,
    pub override_activites: bool,
}
