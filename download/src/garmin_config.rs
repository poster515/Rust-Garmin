
use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct Domain {
    pub domain: String
}

#[derive(Debug, Deserialize, Default)]
pub struct Credentials {
    pub user: String,
    pub secure_password: bool,
    pub password: String
}

#[derive(Debug, Deserialize, Default)]
pub struct DataConfig {
    pub weight_start_date: String,
    pub sleep_start_date: String,
    pub rhr_start_date: String,
    pub monitoring_start_date: String,
    pub download_latest_activities: String,
    pub download_all_activities: String
}

#[derive(Debug, Deserialize, Default)]
pub struct EnabledStats {
    pub monitoring: bool,
    pub steps: bool,
    pub itime: bool,
    pub sleep: bool,
    pub rhr: bool,
    pub weight: bool,
    pub activities: bool
}

#[derive(Debug, Deserialize, Default)]
pub struct GarminConfig {
    pub garmin: Domain,
    pub credentials: Credentials,
    pub data: DataConfig,
    pub enabled_stats: EnabledStats
}
