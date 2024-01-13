
use config::Config;
use reqwest::blocking::Client;
use log::{info, debug};
use url_builder::URLBuilder;
use reqwest;

mod garmin_config;
mod garmin_client;

pub use crate::garmin_client::{GarminClient, ClientTraits};
pub use crate::garmin_config::GarminConfig;

// Class for downloading health data from Garmin Connect.
pub struct DownloadManager {
    
    garmin_connect_user_profile_url: String,
    garmin_connect_wellness_url: String,
    garmin_connect_sleep_daily_url: String,
    garmin_connect_rhr: String,
    garmin_connect_weight_url: String,

    garmin_connect_activity_search_url: String,
    garmin_connect_activity_service_url: String,

    garmin_connect_download_service_url: String,

    garmin_connect_usersummary_url: String,
    garmin_connect_daily_summary_url: String,
    garmin_connect_daily_hydration_url: String,

    download_days_overlap: u32,

    garmin_client: GarminClient,
    garmin_config: GarminConfig
}

pub trait DownloadTraits {
    fn login(&mut self) -> ();
    fn get_activity_types(&mut self) -> ();
    fn get_activities(&mut self) -> ();
    // fn get_activity_types(&mut self) -> Result<bool, DownloadError>;
    // fn get_sleep(&mut self) -> Result<bool, DownloadError>;
    // fn get_resting_heart_rate(&mut self) -> Result<bool, DownloadError>;
    // fn save_to_json_file(&mut self) -> Result<bool, DownloadError>;
}

#[allow(unused_variables)]
impl DownloadManager {
    pub fn new(config: Config) -> DownloadManager {
        DownloadManager {
            garmin_connect_user_profile_url: String::from("/userprofile-service/userprofile"),
            garmin_connect_wellness_url: String::from("/wellness-service/wellness"),
            garmin_connect_sleep_daily_url: String::from("wellness-service/wellness/dailySleepData"),
            garmin_connect_rhr: String::from("userstats-service/wellness/daily"),
            garmin_connect_weight_url: String::from("weight-service/weight/dateRange"),
        
            garmin_connect_activity_search_url: String::from("activitylist-service/activities/search/activities"),
            garmin_connect_activity_service_url: String::from("activity-service/activity"),
        
            garmin_connect_download_service_url: String::from("download-service/files"),
        
            garmin_connect_usersummary_url: String::from("usersummary-service/usersummary"),
            garmin_connect_daily_summary_url: String::from("usersummary-service/usersummary/daily"),
            garmin_connect_daily_hydration_url: String::from("usersummary-service/usersummary/hydration/allData"),
        
            // https://connect.garmin.com/modern/proxy/usersummary-service/usersummary/hydration/allData/2019-11-29
        
            download_days_overlap: 3,  // Existing donloaded data will be redownloaded and overwritten if it is within this number of days of now.
            garmin_client: GarminClient::new(),
            garmin_config: config.try_deserialize().unwrap()
        }
    }
}

impl DownloadTraits for DownloadManager {
    fn login(&mut self) {
        // connect to domain using login url
        let username: &str = &self.garmin_config.credentials.user;
        let password: &str = &self.garmin_config.credentials.password;
        let domain: &str = &self.garmin_config.garmin.domain;

        debug!("login domain: {}, username: {}, password: {}", domain, username, password);

        self.garmin_client.login();
    }

    fn get_activity_types(&mut self) {
        
    }
    fn get_activities(&mut self) {

    }
    // fn get_activity_types(&mut self) -> Result<bool, DownloadError>{

    // }
    // fn get_sleep(&mut self) -> Result<bool, DownloadError>{

    // }
    // fn get_resting_heart_rate(&mut self) -> Result<bool, DownloadError>{

    // }
    // fn save_to_json_file(&mut self) -> Result<bool, DownloadError>{

    // }
}