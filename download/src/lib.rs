
use std::collections::HashMap;
use chrono::{Local, NaiveDateTime, ParseError};
use config::Config;
use getopts::Matches;
use log::{debug, error, info, warn};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

mod garmin_config;
mod garmin_client;
mod garmin_structs;

pub use crate::garmin_client::{GarminClient, ClientTraits};
pub use crate::garmin_config::GarminConfig;
pub use crate::garmin_structs::PersonalInfo;

const GARMIN_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

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

impl DownloadManager {
    pub fn new(config: Config, options: Matches) -> DownloadManager {
        let mut dm = DownloadManager {
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
        };
        // go through options and override anything user specified in CL args
        match options.opt_get::<String>("u") {
            Ok(date) => { match date { Some(d) => { dm.garmin_config.data.summary_date = d;}, None => {}}}, 
            Err(_) => {}
        }
        match options.opt_get::<String>("w") {
            Ok(date) => { match date { Some(d) => { dm.garmin_config.data.weight_start_date = d;}, None => {}}}, 
            Err(_) => {}
        }
        match options.opt_get::<String>("s") {
            Ok(date) => { match date { Some(d) => { dm.garmin_config.data.sleep_start_date = d;}, None => {}}}, 
            Err(_) => {}
        }
        match options.opt_get::<String>("r") {
            Ok(date) => { match date { Some(d) => { dm.garmin_config.data.rhr_start_date = d;}, None => {}}}, 
            Err(_) => {}
        }
        match options.opt_get::<String>("m") {
            Ok(date) => { match date { Some(d) => { dm.garmin_config.data.monitoring_start_date = d;}, None => {}}}, 
            Err(_) => {}
        }
        dm
    }

    pub fn download_all(&mut self) {
        if self.garmin_config.enabled_stats.activities {
            let num_activities = self.garmin_config.activities.num_activities_to_download.parse::<u32>().unwrap();
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

    fn get_download_date(&self, default_date: &str) -> NaiveDateTime{
        // should be used by all date-getters to 1) see if we're 
        // overriding to today and 2) make sure the format is correct if not
        if self.garmin_config.data.download_today_data {
            return Local::now().naive_local();
        }
        let mut temp_date: String = String::from(default_date);
        temp_date.push_str(" 00:00:00");

        match NaiveDateTime::parse_from_str(&temp_date, "%Y-%m-%d %H:%M:%S") {
            Ok(date) => { date },
            Err(e) => panic!("Expected default date in '%Y-%m-%d', format, got: {}, error: {}", default_date, e)
        }
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

        self.save_to_json_file(self.garmin_client.get_last_resp_text(), "activity_types", None, None);
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

        let lookup: Vec<serde_json::Value> = serde_json::from_str(&self.garmin_client.get_last_resp_text()).unwrap();

        for activity in lookup {
            let id = &activity["activityId"];
            let name = &activity["activityName"].to_string().replace('"', "");

            info!("Getting summary for activity {}: {}, on {}", &id, &name, &activity["startTimeLocal"]);

            let activity_string = &activity["startTimeLocal"].to_string().replace('"', "");
            let midnight_string = format!("{}", Local::now().format("%Y-%m-%d 00:00:00"));
            
            let activity = NaiveDateTime::parse_from_str(activity_string, "%Y-%m-%d %H:%M:%S").unwrap();
            let midnight = NaiveDateTime::parse_from_str(&midnight_string, "%Y-%m-%d %H:%M:%S").unwrap();

            if self.garmin_config.data.download_today_data {
                if activity.timestamp_nanos_opt() > midnight.timestamp_nanos_opt() {
                    // download basic info as json, and total activity as FIT file
                    self.get_activity_info(id.to_string().parse::<u64>().unwrap());
                    self.get_activity_details(id.to_string().parse::<u64>().unwrap());
                } else {
                    info!("Ignoring activity '{}' from: {}", &name, activity_string);
                    return;
                }
            }
            self.get_activity_info(id.to_string().parse::<u64>().unwrap());
        }
    }

    pub fn get_activity_info(&mut self, activity_id: u64) {
        // Given specific activity ID, retrieves all basic info as json response body
        let mut endpoint: String = String::from(&self.garmin_connect_activity_service_url);
        endpoint.push_str(&format!("/{}", activity_id));
        if !self.garmin_client.api_request(&endpoint, None){
            warn!("Unable to get API data. Endpoint: {}, activityId: {}", endpoint, activity_id);
            return;
        }
        let lookup: HashMap<String, serde_json::Value> = serde_json::from_str(&self.garmin_client.get_last_resp_text()).unwrap();
        let id = lookup["activityId"].to_string().replace('"', "");
        let name = lookup["activityName"].to_string().replace('"', "");

        let start_datetime = &lookup["summaryDTO"]["startTimeLocal"].to_string().replace("T", " ").replace('"', "");
        match NaiveDateTime::parse_from_str(start_datetime, GARMIN_DATE_FORMAT){
            Ok(date) => {
                self.save_to_json_file(self.garmin_client.get_last_resp_text(), "activities", Some(date),  Some(vec![name, id]));
            }, Err(e) => {
                error!("Could not parse activity datetime: {} using format {} (error: {})", start_datetime, GARMIN_DATE_FORMAT, e);
            }
        }
    }

    pub fn get_activity_details(&mut self, activity_id: u64) {
        // Given specific activity ID, retrieves activity info as FIT file
        let mut endpoint: String = String::from(&self.garmin_connect_download_service_url);
        endpoint.push_str(&format!("/activity/{}", activity_id));
        if !self.garmin_client.api_request(&endpoint, None){
            warn!("Unable to get API data. Endpoint: {}, activityId: {}", endpoint, activity_id);
            return;
        }
        // should have binary data in String - just save to file
        // self.save_to_json_file()
        info!("Need to save contents as binary FIT file!");
    }

    pub fn monitoring(&mut self) {
        // downloads some kind of fit file or something
        let date = self.get_download_date(&self.garmin_config.data.monitoring_start_date);
        let mut endpoint: String = String::from(&self.garmin_connect_download_service_url);
        endpoint.push_str("/wellness/");
        endpoint.push_str(&format!("{}", date.format("%Y-%m-%d")).replace('"', ""));

        self.garmin_client.api_request(&endpoint, None);
        self.save_to_json_file(self.garmin_client.get_last_resp_text(), "monitoring", Some(date), None);
    }

    pub fn get_sleep(&mut self) {
        let date = self.get_download_date(&self.garmin_config.data.sleep_start_date);
        let date_str = String::from(format!("{}", date.format("%Y-%m-%d"))).replace('"', "");
        let mut endpoint: String = String::from(&self.garmin_connect_sleep_daily_url);
        endpoint.push_str(&format!("/{}", &self.get_display_name()));

        let params = HashMap::from([
            ("date", date_str.as_str()),
            ("nonSleepBufferMinutes", "60")
        ]);

        self.garmin_client.api_request(&endpoint, Some(params));
        self.save_to_json_file(self.garmin_client.get_last_resp_text(), "sleep", Some(date), None);
    }

    pub fn get_resting_heart_rate(&mut self) {
        let date = self.get_download_date(&self.garmin_config.data.rhr_start_date);
        let date_str = String::from(format!("{}", date.format("%Y-%m-%d"))).replace('"', "");
        let mut endpoint = String::from(&self.garmin_connect_rhr);
        endpoint.push_str(&format!("/{}", &self.get_display_name()));

        let params = HashMap::from([
            ("fromDate", date_str.as_str()),
            ("untilDate", date_str.as_str()),
            ("metricId", "60")
        ]);

        self.garmin_client.api_request(&endpoint, Some(params));
        self.save_to_json_file(self.garmin_client.get_last_resp_text(), "heartrate", Some(date), None);
    }

    pub fn get_weight(&mut self) {
        let date = self.get_download_date(&self.garmin_config.data.weight_start_date);
        let date_str = String::from(format!("{}", date.format("%Y-%m-%d")).replace('"', ""));
        match self.get_date_in_epoch_ms(&date_str) {
            Ok(epoch_millis) => {
                let endpoint = String::from(&self.garmin_connect_weight_url);
                let params = HashMap::from([
                    ("startDate", date_str.as_str()),
                    ("endDate", date_str.as_str()),
                    ("_", &epoch_millis.as_str())
                ]);
                self.garmin_client.api_request(&endpoint, Some(params));
                self.save_to_json_file(self.garmin_client.get_last_resp_text(), "weight", Some(date), None);
            },
            Err(_) => {}
        }
    }

    pub fn get_summary_day(&mut self) {
        let date = self.get_download_date(&self.garmin_config.data.summary_date);
        let date_str = String::from(format!("{}", date.format("%Y-%m-%d")).replace('"', ""));
        match self.get_date_in_epoch_ms(&date_str) {
            Ok(epoch_millis) => {

                let mut endpoint = String::from(&self.garmin_connect_daily_summary_url);
                endpoint.push_str(&format!("/{}", &self.get_display_name()));

                let params = HashMap::from([
                    ("calendarDate", date_str.as_str()),
                    ("_", epoch_millis.as_str())
                ]);
                self.garmin_client.api_request(&endpoint, Some(params));
                self.save_to_json_file(self.garmin_client.get_last_resp_text(), "day_summary", Some(date), None);

            }, Err(e) => {
                warn!("Unable to properly parse date: {}. Error: {}", &date_str, e);
            }
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

    fn save_to_json_file(&self, 
            data: &str,
            sub_folder: &str,
            activity_date: Option<NaiveDateTime>,
            filename_addons: Option<Vec<String>>) -> (){

        if !self.garmin_config.file.save_to_file {
            info!("Save file config is disabled, ignoring");
        }

        let base_path = String::from(&self.garmin_config.file.file_base_path);

        let file_date: String;
        match activity_date {
            Some(d) => {
                file_date = format!("{}", d.format(&self.garmin_config.file.file_date_format));
            }
            None => {
                file_date = format!("{}", Local::now().format(&self.garmin_config.file.file_date_format));
            }
        }

        let mut filename: String = String::from(format!("{}", file_date.replace('"', "")));
        
        match filename_addons {
            Some(s) => {
                for ext in s {
                    filename.push_str("-");
                    filename.push_str(&ext);
                }
            }, None => {}
        }

        filename.push_str(".json");

        let path = Path::new(&base_path).join(&sub_folder).join(&filename);
        if path.exists() {
            if !self.garmin_config.file.overwrite {
                info!("File: {} exists, but overwrite is disabled, ignoring", path.display());
                return;
            } else {
                info!("File: {} exists, overwriting...", path.display());
            }
        } else {
            info!("Saving file: {} to folder: {}", filename, base_path);
        }

        match File::create(path) {
            Ok(file) => {
                let mut writer = BufWriter::new(file);
                let json_data: HashMap<String, serde_json::Value> = serde_json::from_str(data).unwrap();
                match serde_json::to_writer_pretty(&mut writer, &json_data) {
                    Ok(_) => {
                        match writer.flush() {
                            Ok(_) => {}, 
                            Err(e) => {
                                error!("Error flushing writer: {}", e);
                            }
                        }
                    }, Err(e) => {
                        error!("Error writing json to buffer: {}", e);
                    }
                }
            }, Err(e) => {
                error!("Error creating file: {}", e);
            }
        }
    }
}