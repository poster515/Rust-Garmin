
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::ffi::OsStr;
use chrono::{Local, NaiveDateTime};

use futures::stream;
use config::Config;
use log::{info, error, warn};
use influxdb2::{Client, ClientBuilder};
use influxdb2::models::data_point::DataPoint;
use serde_json::json;

mod influxdb_structs;
use influxdb_structs::InfluxDbConfig;

// actually contains a T but we'll replace that with a 
// space since the DateTime mod can't decode that for
// some reason.
const GARMIN_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

// Class for downloading health data from Garmin Connect.
pub struct UploadManager {
    influx_config: InfluxDbConfig,
    influx_client: Option<Client>,
    bucket: String
}

impl UploadManager {
    pub fn new(config: Config) -> UploadManager {
        UploadManager {
            influx_config: config.try_deserialize().unwrap(),
            influx_client: None,
            bucket: String::new()
        }
    }

    pub fn upload_all(&mut self) {
        self.upload_activity_info();
        self.upload_heart_rate_data();
        self.upload_summary_data();
        self.upload_weight_data();
        self.upload_sleep();
    }

    fn garmin_ts_to_nanos_since_epoch(&self, ts: &str) -> i64 {
        let timestamp = ts.replace('T', " ");
        match NaiveDateTime::parse_from_str(&timestamp, GARMIN_DATE_FORMAT) {
            Ok(timestamp_dt) => { timestamp_dt.timestamp_nanos_opt().unwrap() },
            Err(e) => { 
                error!("Error getting timestamp from: {}, e: {:?}, using current time...", ts, e);
                Local::now().timestamp_nanos_opt().unwrap()
            }
        }
    }

    fn build_client(&mut self) -> bool {
        let url: &str = &self.influx_config.url;
        let org: &str = &self.influx_config.org;
        let token: &str = &self.influx_config.token;

        match ClientBuilder::new(url, org, token).build() {
            Ok(client) => {
                info!("Built influx client: {:?}", client);
                self.influx_client = Some(client);
                return true;
            }, 
            Err(e) => { error!("Unable to create client with:\nurl: {}\norg: {}\ntoken: {}\nerror: {}", url, org, token, e); }
        }
        false
    }

    fn write_data(&mut self, data: Vec<DataPoint>) -> bool {
        match self.influx_client.as_ref() {
            Some(client) => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let future = rt.block_on({
                    client.write(&self.bucket, stream::iter(data))
                });

                match future {
                    Ok(_) => { return true; },
                    Err(e) => { error!("Unable to write data point(s): {:?}", e); return false; }
                }
            }, None => {
                warn!("InfluxDb client not configured yet!");
                if !self.build_client() { return false; }
                return self.write_data(data);
            }
        }
    }

    fn get_extension_from_filename<'a>(&'a self, filename: &'a str) -> Option<&str> {
        Path::new(filename).extension().and_then(OsStr::to_str)
    }

    fn upload_activity_info(&mut self) {
        let base_path = String::from(&self.influx_config.file_base_path);
        let folder = Path::new(&base_path).join("activities");
        if !folder.exists() {
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                if self.get_extension_from_filename(entry.path().to_str().unwrap()) == Some("json") {
                    match File::open(entry.path()) {
                        Ok(file) => {
                            let reader = BufReader::new(file);
                            let activity: HashMap<String, serde_json::Value> = serde_json::from_reader(reader).unwrap();
                            let activity_data = &activity["summaryDTO"];

                            let timestamp = self.garmin_ts_to_nanos_since_epoch(activity_data["startTimeLocal"].as_str().unwrap());

                            let data = DataPoint::builder("activities")
                                .tag("type",                        activity["activityName"].to_string())
                                .field("activityTrainingLoad",      activity_data["activityTrainingLoad"].as_f64().unwrap())
                                .field("anaerobicTrainingEffect",   activity_data["anaerobicTrainingEffect"].as_f64().unwrap())
                                .field("averageHR",                 activity_data["averageHR"].as_f64().unwrap())
                                .field("averageSpeed",              activity_data["averageSpeed"].as_f64().unwrap())
                                .field("avgRespirationRate",        activity_data["avgRespirationRate"].as_f64().unwrap())
                                .field("bmrCalories",               activity_data["bmrCalories"].as_f64().unwrap())
                                .field("calories",                  activity_data["calories"].as_f64().unwrap())
                                .field("distance",                  activity_data["distance"].as_f64().unwrap())
                                .field("duration",                  activity_data["duration"].as_f64().unwrap())
                                .field("elapsedDuration",           activity_data["elapsedDuration"].as_f64().unwrap())
                                .field("maxHR",                     activity_data["maxHR"].as_f64().unwrap())
                                .field("maxRespirationRate",        activity_data["maxRespirationRate"].as_f64().unwrap())
                                .field("minActivityLapDuration",    activity_data["minActivityLapDuration"].as_f64().unwrap())
                                .field("minRespirationRate",        activity_data["minRespirationRate"].as_f64().unwrap())
                                .field("moderateIntensityMinutes",  activity_data["moderateIntensityMinutes"].as_f64().unwrap())
                                .field("movingDuration",            activity_data["movingDuration"].as_f64().unwrap())
                                .field("steps",                     activity_data["steps"].as_i64().unwrap())
                                .field("trainingEffect",            activity_data["trainingEffect"].as_f64().unwrap())
                                .field("vigorousIntensityMinutes",  activity_data["vigorousIntensityMinutes"].as_f64().unwrap())
                                .timestamp(timestamp)
                                .build();

                            self.write_data(vec![data.unwrap()]);

                        }, Err(e) => { error!("Failed to open file {:?}, error: {}", entry.path(), e); }
                    }
                } else if self.get_extension_from_filename(entry.path().to_str().unwrap()) == Some("fit") {
                    let mut fp = File::open(entry.path()).unwrap();
                    for data in fitparser::from_reader(&mut fp).unwrap() {
                        // print the data in FIT file
                        println!("{:#?}", data);
                    }
                }
            }
        }
    }
    fn upload_sleep(&mut self) {
        let base_path = String::from(&self.influx_config.file_base_path);
        let folder = Path::new(&base_path).join("sleep");
        if !folder.exists() {
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                match File::open(entry.path()) {
                    Ok(file) => {
                        let reader = BufReader::new(file);
                        let sleep: HashMap<String, serde_json::Value> = serde_json::from_reader(reader).unwrap();
                        
                        let restless_moments = json!(sleep["sleepRestlessMoments"]);
                        let sleep_levels = json!(sleep["sleepLevels"]);
                        let hrv = json!(sleep["hrv"]);
                        let sleep_stress = json!(sleep["sleepStress"]);
                        let sleep_movement = json!(sleep["sleepMovement"]);
                    }, Err(e) => { error!("Unable to open file: {}, error: {:?}", entry.path().to_str().unwrap(), e) }
                }
            }
        }
    }
    fn upload_heart_rate_data(&mut self) {
        let base_path = String::from(&self.influx_config.file_base_path);
        let folder = Path::new(&base_path).join("heartrate");
        if !folder.exists() {
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                println!("{:?}", entry.path());
            }
        }
    }
    fn upload_weight_data(&mut self) {
        let base_path = String::from(&self.influx_config.file_base_path);
        let folder = Path::new(&base_path).join("weight");
        if !folder.exists() {
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                println!("{:?}", entry.path());
            }
        }
    }
    fn upload_summary_data(&mut self) {
        let base_path = String::from(&self.influx_config.file_base_path);
        let folder = Path::new(&base_path).join("day_summary");
        if !folder.exists() {
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                println!("{:?}", entry.path());
            }
        }
    }
}