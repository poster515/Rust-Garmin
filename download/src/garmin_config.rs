
use chrono::{DateTime, Local};
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
pub struct EnabledStats {
    monitoring: bool,
    steps: bool,
    itime: bool,
    sleep: bool,
    rhr: bool,
    weight: bool,
    activities: bool
}

#[derive(Debug, Deserialize, Default)]
pub struct DataConfig {
    weight_start_date: DateTime<Local>,
    sleep_start_date: DateTime<Local>,
    rhr_start_date: DateTime<Local>,
    monitoring_start_date: DateTime<Local>,
    download_latest_activities: DateTime<Local>,
    download_all_activities: DateTime<Local>
}

#[derive(Debug, Deserialize, Default)]
pub struct GarminConfig {
    pub domain: Domain,
    pub credentials: Credentials,
    data_config: DataConfig,
    enabled_stats: EnabledStats
}
