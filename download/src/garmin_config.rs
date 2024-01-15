
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
    pub summary_date: String,
    pub weight_start_date: String,
    pub sleep_start_date: String,
    pub rhr_start_date: String,
    pub monitoring_start_date: String,
    pub download_today_data: bool,
    pub num_activities_to_download: String,
    pub download_all_activities: String
}

#[derive(Debug, Deserialize, Default)]
pub struct EnabledStats {
    pub daily_summary: bool,
    pub monitoring: bool,
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
