
use std::collections::HashMap;
use chrono::{Local, NaiveDateTime, ParseError};
use config::Config;
use log::{debug, error, info};

mod garmin_config;
mod garmin_client;
mod garmin_structs;

pub use crate::garmin_client::{GarminClient, ClientTraits};
pub use crate::garmin_config::GarminConfig;
pub use crate::garmin_structs::PersonalInfo;

// Class for downloading health data from Garmin Connect.
#[allow(dead_code)]
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

    garmin_user_profile_url: String,

    download_days_overlap: u32,

    garmin_client: GarminClient,
    garmin_config: GarminConfig,
    personal_info: PersonalInfo,
    full_name: String,
    display_name: String
}

#[allow(dead_code)]
impl DownloadManager {
    pub fn new(config: Config) -> DownloadManager {
        DownloadManager {
            garmin_connect_user_profile_url: String::from("userprofile-service/userprofile"),

            garmin_connect_wellness_url: String::from("wellness-service/wellness"),
            garmin_connect_sleep_daily_url: String::from("wellness-service/wellness/dailySleepData"),
            garmin_connect_rhr: String::from("userstats-service/wellness/daily"),
            garmin_connect_weight_url: String::from("weight-service/weight/dateRange"),
        
            garmin_connect_activity_search_url: String::from("activitylist-service/activities/search/activities"),
            garmin_connect_activity_service_url: String::from("activity-service/activity"),
        
            garmin_connect_download_service_url: String::from("download-service/files"),
        
            garmin_connect_usersummary_url: String::from("usersummary-service/usersummary"),
            garmin_connect_daily_summary_url: String::from("usersummary-service/usersummary/daily"),
            garmin_connect_daily_hydration_url: String::from("usersummary-service/usersummary/hydration/allData"),

            garmin_user_profile_url: String::from("userprofile-service/socialProfile"),

            download_days_overlap: 3,  // Existing donloaded data will be redownloaded and overwritten if it is within this number of days of now.
            garmin_client: GarminClient::new(),
            garmin_config: config.try_deserialize().unwrap(),
            personal_info: Default::default(),
            full_name: String::new(),
            display_name: String::new()
        }
    }

    pub fn download(&mut self) {
        if self.garmin_config.enabled_stats.activities {
            let num_activities = self.garmin_config.data.num_activities_to_download.parse::<u32>().unwrap();
            self.get_activity_summaries(num_activities);
        }
        if self.garmin_config.enabled_stats.sleep {
            self.get_sleep();
        }
        if self.garmin_config.enabled_stats.rhr {
            self.get_resting_heart_rate();
        }
        if self.garmin_config.enabled_stats.weight {
            self.get_weight();
        }
        if self.garmin_config.enabled_stats.daily_summary {
            self.get_summary_day();
        }        
    }

    pub fn get_user_profile(&mut self){
        // response will contain displayName and fullName
        self.garmin_client.api_request(&self.garmin_user_profile_url, None);

        let lookup: HashMap<String, serde_json::Value> = serde_json::from_str(&self.garmin_client.get_last_resp_text()).unwrap();

        if lookup.contains_key("displayName"){
            self.display_name = lookup["displayName"].to_string().replace('"', "");
            info!("Display name: '{}'", self.display_name);
        }

        if lookup.contains_key("fullName"){
            self.full_name = lookup["fullName"].to_string().replace('"', "");
            info!("Full name: '{}'", self.full_name);
        }
    }

    pub fn get_display_name(&mut self) -> String {
        if self.display_name.len() == 0 {
            self.get_user_profile();
        }
        return String::from(&self.display_name);
    }

    pub fn get_full_name(&mut self) -> String {
        if self.full_name.len() == 0 {
            self.get_user_profile();
        }
        return String::from(&self.full_name);
    }

    fn get_download_date(&self, default_date: &str) -> String{
        if self.garmin_config.data.download_today_data {
            return format!("{}", Local::now().format("%Y-%m-%d"));
        }
        String::from(default_date)
    }

    pub fn login(&mut self) {
        // connect to domain using login url
        let username: &str = &self.garmin_config.credentials.user;
        let password: &str = &self.garmin_config.credentials.password;
        let domain: &str = &self.garmin_config.garmin.domain;

        debug!("login domain: {}, username: {}, password: {}", domain, username, password);

        self.garmin_client.login(username, password);
    }

    pub fn get_personal_info(&mut self) {
        let mut personal_info_endpoint: String = String::from(&self.garmin_connect_user_profile_url);
        personal_info_endpoint.push_str("/personal-information");

        if !self.garmin_client.api_request(&personal_info_endpoint, None) {
            return
        }

        // deserialize into struct
        self.personal_info = serde_json::from_str(self.garmin_client.get_last_resp_text()).unwrap();
        info!("Got personal info. \nuserId: {}\nbirthday: {}\nemail: {}\nage: {}",
            &self.personal_info.biometricProfile.userId,
            &self.personal_info.userInfo.birthDate,
            &self.personal_info.userInfo.email,
            &self.personal_info.userInfo.age
        )
    }

    pub fn get_activity_types(&mut self) {
        // retrieves all possible activity types from Garmin. Included activityTypeIds for each.
        let mut endpoint: String = String::from(&self.garmin_connect_activity_service_url);
        endpoint.push_str("/activityTypes");
        self.garmin_client.api_request(&endpoint, None);
    }

    pub fn get_activity_summaries(&mut self, activity_count: u32) {
        // get high level activity summary, each entry contains activity ID that
        // can be used to get more specific info
        let endpoint: String = String::from(&self.garmin_connect_activity_search_url);
        let count = format!("{}", activity_count);
        let params = HashMap::from([
            ("start", "0"),
            ("limit", &count),
        ]);
        self.garmin_client.api_request(&endpoint, Some(params));

        // let lookup: HashMap<String, serde_json::Value> = serde_json::from_str(&self.garmin_client.get_last_resp_text()).unwrap();

        // TODO: check for dates since midnight if 'download_today_data' is true, and download those
    }

    pub fn get_activity_info(&mut self, activity_id: u64) {
        // Given specific activity ID, retrieves all info
        let mut endpoint: String = String::from(&self.garmin_connect_activity_service_url);
        endpoint.push_str(&format!("/{}", activity_id));
        self.garmin_client.api_request(&endpoint, None);
    }

    pub fn monitoring(&mut self) {
        // url = f'{self.garmin_connect_download_service_url}/wellness/{date.strftime("%Y-%m-%d")}'
        let date_str = self.get_download_date(&self.garmin_config.data.monitoring_start_date);
        let mut endpoint: String = String::from(&self.garmin_connect_download_service_url);
        endpoint.push_str("/wellness/");
        endpoint.push_str(&date_str);

        self.garmin_client.api_request(&endpoint, None);
    }

    pub fn get_sleep(&mut self) {
        let date_str = self.get_download_date(&self.garmin_config.data.sleep_start_date);
        let mut endpoint: String = String::from(&self.garmin_connect_sleep_daily_url);
        endpoint.push_str(&format!("/{}", &self.display_name));

        let params = HashMap::from([
            ("date", date_str.as_str()),
            ("nonSleepBufferMinutes", "60")
        ]);

        self.garmin_client.api_request(&endpoint, Some(params));
    }

    pub fn get_resting_heart_rate(&mut self) {
        let date_str = self.get_download_date(&self.garmin_config.data.rhr_start_date);
        let mut endpoint = String::from(&self.garmin_connect_rhr);
        endpoint.push_str(&format!("/{}", &self.display_name));

        let params = HashMap::from([
            ("fromDate", date_str.as_str()),
            ("untilDate", date_str.as_str()),
            ("metricId", "60")
        ]);

        self.garmin_client.api_request(&endpoint, Some(params));
    }

    pub fn get_weight(&mut self) {
        let date_str = self.get_download_date(&self.garmin_config.data.weight_start_date);
        match self.get_date_in_epoch_ms(&date_str) {
            Ok(epoch_millis) => {
                let endpoint = String::from(&self.garmin_connect_weight_url);
                let params = HashMap::from([
                    ("startDate", date_str.as_str()),
                    ("endDate", date_str.as_str()),
                    ("_", &epoch_millis.as_str())
                ]);
                self.garmin_client.api_request(&endpoint, Some(params));
            },
            Err(_) => {}
        }
    }

    pub fn get_summary_day(&mut self) {
        let date_str = self.get_download_date(&self.garmin_config.data.summary_date);

        match self.get_date_in_epoch_ms(&date_str) {
            Ok(epoch_millis) => {

                let mut endpoint = String::from(&self.garmin_connect_daily_summary_url);
                endpoint.push_str(&format!("/{}", &self.display_name));

                let params = HashMap::from([
                    ("calendarDate", date_str.as_str()),
                    ("_", epoch_millis.as_str())
                ]);
                self.garmin_client.api_request(&endpoint, Some(params));

            }, Err(_) => {}
        }
    }

    fn get_date_in_epoch_ms(&self, date_str: &str) -> Result<String, ParseError> {
        
        let mut qualified_date = String::from(date_str);
        qualified_date.push_str(" 00:00:00");
        let datetime_result = NaiveDateTime::parse_from_str(&qualified_date, "%Y-%m-%d %H:%M:%S");
        match datetime_result {
            Ok(datetime) => {
                let epoch_millis = format!("{}", datetime.timestamp_millis());
                return Ok(epoch_millis)

            }, Err(e) => {
                error!("Unable to parse config datetime into '%Y-%m-%d': {}", date_str);
                Err(e)
            }
        }
    }
    // fn save_to_json_file(&mut self) -> Result<bool, DownloadError>{

    // }
}