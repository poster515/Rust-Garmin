
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use chrono::{Local, NaiveDateTime, ParseError, Days};
use config::Config;
use getopts::Matches;
use log::{debug, error, info, warn};
use std::path::Path;

use garmin_client;

mod garmin_config;
mod garmin_structs;

pub use crate::garmin_client::{GarminClient, ClientTraits, SESSION_FILE};
pub use crate::garmin_config::GarminConfig;
pub use crate::garmin_structs::PersonalInfo;


/// Class for downloading health data from Garmin Connect.
/// This class requires the garmin_client crate to provide authentication
/// and authorization for to the garmin backend, and contains all the 
/// target urls for downloading various health and activity data.
///
/// This class is intended to be provided a Config item based on the 
/// garmin_structs.rs (see ../config/garmin_config.json for an example),
/// and is intended to be run on a scheduled basis, although future
/// session management improvement will allow this to be more of a 
/// command line utility.
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

    garmin_client: GarminClient,
    garmin_config: GarminConfig,
    personal_info: PersonalInfo,
    full_name: String,
    display_name: String
}

impl DownloadManager {
    /// This constructor expects a config and optional Matches overrides, allowing for
    /// a more CLI-friendly control flow.
    ///
    /// 'options' parameters:<br />
    ///     &nbsp;&nbsp;&nbsp;&nbsp;"u": "YYY-MM-DD" -> overrides the download date for summary info (JSON)<br />
    ///     &nbsp;&nbsp;&nbsp;&nbsp;"w": "YYY-MM-DD" -> overrides the download date for weight info (JSON)<br />
    ///     &nbsp;&nbsp;&nbsp;&nbsp;"s": "YYY-MM-DD" -> overrides the download date for sleeep info (JSON)<br />
    ///     &nbsp;&nbsp;&nbsp;&nbsp;"r": "YYY-MM-DD" -> overrides the download date for heart_rate info (JSON)<br />
    ///     &nbsp;&nbsp;&nbsp;&nbsp;"m": "YYY-MM-DD" -> overrides the download date for monitoring data (FIT file)<br />
    /// 
    /// Each API call saves the response url and text in case users want more info from the call. These are saved after
    /// the most recent call (i.e., no API response 'history' included) and overwritten with each call. 
    pub fn new(config: Config, options: Option<Matches>) -> DownloadManager {
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

            garmin_client: GarminClient::new(),
            garmin_config: config.try_deserialize().unwrap(),
            personal_info: Default::default(),
            full_name: String::new(),
            display_name: String::new()
        };

        if let Some(options) = options {
            // go through options and override anything user specified in CL args
            if let Ok(Some(date)) = options.opt_get::<String>("u") {
                dm.garmin_config.data.summary_date = date;
                dm.garmin_config.enabled_stats.daily_summary = true; 
            }
            if let Ok(Some(date)) = options.opt_get::<String>("w") {
                dm.garmin_config.data.weight_start_date = date;
                dm.garmin_config.enabled_stats.weight = true;
            }
            if let Ok(Some(date)) = options.opt_get::<String>("s") {
                dm.garmin_config.data.sleep_start_date = date;
                dm.garmin_config.enabled_stats.sleep = true;
            }
            if let Ok(Some(date)) = options.opt_get::<String>("r") {
                dm.garmin_config.data.rhr_start_date = date;
                dm.garmin_config.enabled_stats.rhr = true;
            }
            if let Ok(Some(date)) = options.opt_get::<String>("o") {
                dm.garmin_config.data.hydration_start_date = date;
                dm.garmin_config.enabled_stats.hydration = true;
            }
            if let Ok(Some(date)) = options.opt_get::<String>("m") {
                dm.garmin_config.data.monitoring_start_date = date;
                dm.garmin_config.enabled_stats.monitoring = true;
            }
            if let Ok(Some(date)) = options.opt_get::<String>("a") {
                dm.garmin_config.data.activity_start_date = date;
                dm.garmin_config.enabled_stats.activities = true;
            }
        }
        if dm.garmin_config.data.download_today_data {
            dm.garmin_config.data.num_days_from_start_date = 1;
        }
        dm
    }

    /// Downloads all data enabled in config provided in 'new()'
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
        if self.garmin_config.enabled_stats.monitoring {
            self.monitoring();
        }
        if self.garmin_config.enabled_stats.hydration {
            self.get_hydration();
        }
    }

    pub fn get_last_resp_text(&self) -> &str {
        &self.garmin_client.get_last_resp_text()
    }

    /// Retrives user profile, which includes fields like displayName and fullName.
    ///
    /// User can retrieve full response text via self.get_last_resp_text() if needed.
    pub fn get_user_profile(&mut self){

        // check session file for displayName and fullName
        // if session file exists: open, append, save user profile info
        match fs::read_to_string(&SESSION_FILE) {
            Ok(file_contents) => {
                let map: HashMap<String, serde_json::Value> = serde_json::from_str(&file_contents).unwrap();

                if map.contains_key("displayName") && map.contains_key("fullName"){
                    self.display_name = map["displayName"].to_string().replace('"', "");
                    self.full_name = map["fullName"].to_string().replace('"', "");
                    info!("Found display name in session file: '{}'", self.display_name);
                    info!("Found full name in session file: '{}'", self.full_name);
                    return;
                } else {
                    info!("Unable to locate user profile from session file, requesting...");
                }
            }, Err(_) => {
                warn!("Unable to locate session file, did you login yet?");
            }
        }

        // response will contain displayName and fullName
        self.garmin_client.api_request(&self.garmin_user_profile_url, None, true, None);

        let response_text = self.get_last_resp_text();
        if response_text.len() == 0 {
            warn!("Got empty response from API, unable to get user profile. Are you using the latest client version?");
            return;
        }

        let lookup: HashMap<String, serde_json::Value> = serde_json::from_str(response_text).unwrap();

        if lookup.contains_key("displayName"){
            self.display_name = lookup["displayName"].to_string().replace('"', "");
            info!("Display name: '{}'", self.display_name);
        }

        if lookup.contains_key("fullName"){
            self.full_name = lookup["fullName"].to_string().replace('"', "");
            info!("Full name: '{}'", self.full_name);
        }

        // if session file exists: open, append, save user profile info
        match fs::read_to_string(&SESSION_FILE) {
            Ok(file_contents) => {
                let mut map: HashMap<String, serde_json::Value> = serde_json::from_str(&file_contents).unwrap();
                map.extend(lookup);

                let file = File::create(&SESSION_FILE).unwrap(); 
                let mut writer = BufWriter::new(file);
                if let Ok(_) = serde_json::to_writer_pretty(&mut writer, &map) {
                    match writer.flush() {
                        Ok(_) => { }, 
                        Err(e) => { error!("Error flushing writer: {}", e); }
                    }
                }
            }, Err(e) => { info!("Unable opening garmin_client session file: {}", e); }
        }
    }

    /// Retrieves the user's display name.
    pub fn get_display_name(&mut self) -> String {
        if self.display_name.len() == 0 {
            self.get_user_profile();
        }
        return String::from(&self.display_name);
    }

    /// Retrieves the user's full name.
    pub fn get_full_name(&mut self) -> String {
        if self.full_name.len() == 0 {
            self.get_user_profile();
        }
        return String::from(&self.full_name);
    }

    fn get_download_date(&self, default_date: &str, day_offset: u64) -> NaiveDateTime {
        // should be used by all date-getters to 1) see if we're 
        // overriding to today and 2) make sure the format is correct if not
        if self.garmin_config.data.download_today_data {
            info!("download_today_data set - ignoring any config or command line dates");
            return Local::now().naive_local();
        }
        let mut temp_date: String = String::from(default_date);
        temp_date.push_str(" 00:00:00");

        match NaiveDateTime::parse_from_str(&temp_date, "%Y-%m-%d %H:%M:%S") {
            Ok(date) => { date.checked_add_days(Days::new(day_offset)).unwrap() },
            Err(e) => panic!("Expected default date in '%Y-%m-%d', format, got: {}, error: {}", default_date, e)
        }
    }

    /// Logs in using the configured username and password.
    pub fn login(&mut self) {
        // connect to domain using login url
        let username: &str = &self.garmin_config.credentials.user;
        let password: &str = &self.garmin_config.credentials.password;
        let domain: &str = &self.garmin_config.garmin.domain;

        debug!("login domain: {}, username: {}, password: {}", domain, username, password);

        self.garmin_client.login(username, password);
    }

    /// Retrieves and prints the user's personal info (e.g., userId, birthday, email, etc)
    pub fn get_personal_info(&mut self) {
        let mut personal_info_endpoint: String = String::from(&self.garmin_connect_user_profile_url);
        personal_info_endpoint.push_str("/personal-information");

        if !self.garmin_client.api_request(&personal_info_endpoint, None, true, None) {
            return
        }

        let response_text = self.get_last_resp_text();
        if response_text.len() == 0 {
            warn!("Got empty response from API, unable to get personal info");
            return;
        }

        // deserialize into struct
        self.personal_info = serde_json::from_str(response_text).unwrap();
        info!("Got personal info. \nuserId: {}\nbirthday: {}\nemail: {}\nage: {}",
            &self.personal_info.biometricProfile.userId,
            &self.personal_info.userInfo.birthDate,
            &self.personal_info.userInfo.email,
            &self.personal_info.userInfo.age
        )
    }

    /// Retrieves the activity: activityId mapping from garmin.
    pub fn get_activity_types(&mut self) {
        // retrieves all possible activity types from Garmin. Included activityTypeIds for each.
        let mut endpoint: String = String::from(&self.garmin_connect_activity_service_url);
        endpoint.push_str("/activityTypes");
        let filename = self.build_file_name("activity_types", None, None, ".json");
        self.garmin_client.api_request(&endpoint, None, true, filename);
    }

    /// Downloads last activity_count JSON summary and associated FIT files.
    ///
    /// If this DownloadManager was configured with 'download_today_data': true
    /// then only those activities that occurred today will be actually saved.
    pub fn get_activity_summaries(&mut self, activity_count: u32) {
        // get high level activity summary, each entry contains activity ID that
        // can be used to get more specific info
        if activity_count == 0 {
            warn!("User requested 0 activities, check config");
            return;
        }
        let endpoint: String = String::from(&self.garmin_connect_activity_search_url);
        let count = format!("{}", activity_count);
        let params = HashMap::from([
            ("start", "0"),
            ("limit", &count),
        ]);
        self.garmin_client.api_request(&endpoint, Some(params), true, None);

        let response_text = self.get_last_resp_text();
        if response_text.len() == 0 {
            warn!("Got empty response from API, unable to get summaries for last {} activities", activity_count);
            return;
        }

        let lookup: Vec<serde_json::Value> = serde_json::from_str(response_text).unwrap();

        for activity in lookup {
            let id = &activity["activityId"];
            let name = &activity["activityName"].to_string().replace('"', "");
            let activity_string = &activity["startTimeLocal"].to_string().replace('"', "");
            let activity_date = NaiveDateTime::parse_from_str(activity_string, "%Y-%m-%d %H:%M:%S").unwrap();

            let mut start_string: Option<String> = None;

            if self.garmin_config.data.download_today_data {
                // check if the activity started today
                start_string = Some(format!("{}", Local::now().format("%Y-%m-%d 00:00:00")));

            } else if !self.garmin_config.activities.save_regardless_of_date {
                // check if activity started on the date specified
                start_string = Some(format!("{} 00:00:00", self.garmin_config.data.activity_start_date).replace('"', ""));
            }

            if let Some(start_string) = start_string {
                let start = NaiveDateTime::parse_from_str(&start_string, "%Y-%m-%d %H:%M:%S").unwrap();
                let end = start.clone().checked_add_days(Days::new(self.garmin_config.data.num_days_from_start_date)).unwrap();

                if (activity_date.timestamp_nanos_opt() < start.timestamp_nanos_opt()) ||
                (activity_date.timestamp_nanos_opt() >= end.timestamp_nanos_opt()) {
                    info!("Ignoring activity '{}' from: {}", &name, activity_string);
                    continue;
                }
            }

            self.get_activity_info(id.to_string().parse::<u64>().unwrap());
            self.get_activity_details(id.to_string().parse::<u64>().unwrap());
        }
    }

    /// Downloads JSON info for a particular activity ID, as JSON. 
    ///
    /// While this DownloadManager provides a progammatic way of doing 
    /// this, you can go to your activity on the garmin connect website,
    /// get the id via the url, and provide that ID to this function.
    pub fn get_activity_info(&mut self, activity_id: u64) {
        // Given specific activity ID, retrieves all basic info as json response body
        let mut endpoint: String = String::from(&self.garmin_connect_activity_service_url);
        endpoint.push_str(&format!("/{}", activity_id));

        info!("====================================================");
        info!("Getting info for activity {:}", &activity_id);

        let filename = self.build_file_name("activities", None, Some(vec![activity_id.to_string()]), ".json");
        self.garmin_client.api_request(&endpoint, None, true, filename);
    }

    /// Downloads FIT file for a particular activity ID. 
    ///
    /// While this DownloadManager provides a progammatic way of doing 
    /// this, you can go to your activity on the garmin connect website,
    /// get the id via the url, and provide that ID to this function.
    pub fn get_activity_details(&mut self, activity_id: u64) {
        // activity data downloaded as a zip file containing the fit file.
        let mut endpoint: String = String::from(&self.garmin_connect_download_service_url);
        endpoint.push_str(&format!("/activity/{}", activity_id));

        info!("====================================================");
        info!("Getting details for activity {:}", &activity_id);

        let filename = self.build_file_name("activities", None, Some(vec![activity_id.to_string()]), ".zip");
        self.garmin_client.api_request(&endpoint, None, false, filename);
    }

    /// Downloads FIT file info for the configured monitoring date.
    pub fn monitoring(&mut self) {
        // monitoring data downloaded as a zip file containing the fit file.
        for i in 0..self.garmin_config.data.num_days_from_start_date {
            let date = self.get_download_date(&self.garmin_config.data.monitoring_start_date, i);
            let mut endpoint: String = String::from(&self.garmin_connect_download_service_url);
            endpoint.push_str("/wellness/");
            endpoint.push_str(&format!("{}", date.format("%Y-%m-%d")).replace('"', ""));
            
            let filename = self.build_file_name("monitoring", Some(date), None, ".zip");
            self.garmin_client.api_request(&endpoint, None, false, filename);
        }
    }

    /// Downloads sleep info as JSON file, for the configured sleep date.
    pub fn get_sleep(&mut self) {
        for i in 0..self.garmin_config.data.num_days_from_start_date {
            let date = self.get_download_date(&self.garmin_config.data.sleep_start_date, i);
            let date_str = String::from(format!("{}", date.format("%Y-%m-%d"))).replace('"', "");
            let mut endpoint: String = String::from(&self.garmin_connect_sleep_daily_url);
            endpoint.push_str(&format!("/{}", &self.get_display_name()));

            let params = HashMap::from([
                ("date", date_str.as_str()),
                ("nonSleepBufferMinutes", "60")
            ]);

            let filename = self.build_file_name("sleep", Some(date), None, ".json");
            self.garmin_client.api_request(&endpoint, Some(params), true, filename);
        }
    }

    /// Downloads resting heart rate info as JSON file, for the configured date.
    pub fn get_resting_heart_rate(&mut self) {
        for i in 0..self.garmin_config.data.num_days_from_start_date {
            let date = self.get_download_date(&self.garmin_config.data.rhr_start_date, i);
            let date_str = String::from(format!("{}", date.format("%Y-%m-%d"))).replace('"', "");
            let mut endpoint = String::from(&self.garmin_connect_rhr);
            endpoint.push_str(&format!("/{}", &self.get_display_name()));

            let params = HashMap::from([
                ("fromDate", date_str.as_str()),
                ("untilDate", date_str.as_str()),
                ("metricId", "60")
            ]);
            let filename = self.build_file_name("heartrate", Some(date), None, ".json");
            self.garmin_client.api_request(&endpoint, Some(params), true, filename);
        }
    }

     /// Downloads weight info as JSON file, for the configured date.
    pub fn get_weight(&mut self) {
        for i in 0..self.garmin_config.data.num_days_from_start_date {
            let date = self.get_download_date(&self.garmin_config.data.weight_start_date, i);
            let date_str = String::from(format!("{}", date.format("%Y-%m-%d")).replace('"', ""));
            match self.get_date_in_epoch_ms(&date_str) {
                Ok(epoch_millis) => {
                    let endpoint = String::from(&self.garmin_connect_weight_url);
                    let params = HashMap::from([
                        ("startDate", date_str.as_str()),
                        ("endDate", date_str.as_str()),
                        ("_", &epoch_millis.as_str())
                    ]);
                    let filename = self.build_file_name("weight", Some(date), None, ".json");
                    self.garmin_client.api_request(&endpoint, Some(params), true, filename);
                }, Err(_) => {}
            }
        }
    }

     /// Downloads summary info as JSON file, for the configured date.
    pub fn get_summary_day(&mut self) {
        for i in 0..self.garmin_config.data.num_days_from_start_date {
            let date = self.get_download_date(&self.garmin_config.data.summary_date, i);
            let date_str = String::from(format!("{}", date.format("%Y-%m-%d")).replace('"', ""));
            match self.get_date_in_epoch_ms(&date_str) {
                Ok(epoch_millis) => {

                    let mut endpoint = String::from(&self.garmin_connect_daily_summary_url);
                    endpoint.push_str(&format!("/{}", &self.get_display_name()));

                    let params = HashMap::from([
                        ("calendarDate", date_str.as_str()),
                        ("_", epoch_millis.as_str())
                    ]);
                    let filename = self.build_file_name("day_summary", Some(date), None, ".json");
                    self.garmin_client.api_request(&endpoint, Some(params), true, filename);

                }, Err(e) => {
                    warn!("Unable to properly parse date: {}. Error: {}", &date_str, e);
                }
            }
        }
    }

     /// Downloads hydration info as JSON file, for the configured date.
    pub fn get_hydration(&mut self) {
        for i in 0..self.garmin_config.data.num_days_from_start_date {
            let date = self.get_download_date(&self.garmin_config.data.hydration_start_date, i);
            let date_str = String::from(format!("{}", date.format("%Y-%m-%d")).replace('"', ""));

            let mut endpoint = String::from(&self.garmin_connect_daily_hydration_url);
            endpoint.push_str(&format!("/hydration_{}", &date_str));

            let filename = self.build_file_name("hydration", Some(date), None, ".json");
            self.garmin_client.api_request(&endpoint, None, true, filename);
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

    fn build_file_name(&self,
            sub_folder: &str,
            activity_date: Option<NaiveDateTime>,
            filename_addons: Option<Vec<String>>,
            extension: &str) -> Option<String> {

        if !self.garmin_config.file.save_to_file {
            info!("Save file config is disabled, ignoring");
            return None;
        }

        let base_path = String::from(&self.garmin_config.file.file_base_path);

        let mut filename: String;
        match activity_date {
            Some(d) => {
                filename = format!("{}", d.format(&self.garmin_config.file.file_date_format)).replace('"', "");
            }
            None => {
                filename = format!("{}", Local::now().format(&self.garmin_config.file.file_date_format)).replace('"', "");
            }
        }

        if let Some(s) = filename_addons {
            for addon in s {
                filename.push_str("-");
                filename.push_str(&addon);
            }
        }

        filename.push_str(extension);

        let path = Path::new(&base_path).join(&sub_folder).join(&filename);
        if path.exists() {
            if !self.garmin_config.file.overwrite {
                info!("File: {} exists, but overwrite is disabled, ignoring", path.display());
                return None;
            } else {
                info!("File: {} exists, overwriting with any received data...", path.display());
            }
        } else {
            info!("Saving any received data to file: {}", path.display())
        }
        Some(path.to_str().unwrap().to_string())
    }
}