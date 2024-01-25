
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::ffi::OsStr;
use chrono::{Local, NaiveDateTime, DateTime};

use futures::stream;
use config::Config;
use log::{info, error, warn};
use influxdb2::{Client, ClientBuilder};
use influxdb2::models::data_point::DataPoint;
use regex::Regex;

mod influxdb_structs;
use influxdb_structs::InfluxDbConfig;

mod msg_type_map;

// actually contains a T but we'll replace that with a 
// space since the DateTime mod can't decode that for
// some reason.
const GARMIN_JSON_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";
const GARMIN_FIT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S %z";
const GARMIN_EPOCH_OFFSET: i64 = 631065600;

// Class for downloading health data from Garmin Connect.
pub struct UploadManager {
    influx_config: InfluxDbConfig,
    influx_client: Option<Client>
}

impl UploadManager {
    pub fn new(config: Config) -> UploadManager {
        UploadManager {
            influx_config: config.try_deserialize().unwrap(),
            influx_client: None
        }
    }

    pub fn upload_all(&mut self) {
        if self.influx_config.upload_json_files {
            self.upload_activity_info();
            self.upload_heart_rate_data();
            self.upload_summary_data();
            self.upload_weight_data();
            self.upload_sleep();
        } else {
            info!("Ignoring JSON file uploads");
        }

        if self.influx_config.upload_fit_files {
            self.upload_monitoring();
            self.upload_activity_details();
        } else {
            info!("Ignoring FIT file uploads");
        }
    }

    fn garmin_ts_to_nanos_since_epoch(&self, ts: &str) -> i64 {
        let timestamp = ts.replace('T', " ");
        match NaiveDateTime::parse_from_str(&timestamp, GARMIN_JSON_DATE_FORMAT) {
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
                true
            },
            Err(e) => { 
                error!("Unable to create client with:\nurl: {}\norg: {}\ntoken: {}\nerror: {}", url, org, token, e); 
                false
            }
        }
    }

    fn write_data(&mut self, data: Vec<DataPoint>) -> bool {
        match self.influx_client.as_ref() {
            Some(client) => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let future = rt.block_on({
                    client.write(&self.influx_config.bucket, stream::iter(data))
                });

                match future {
                    Ok(_) => { info!("Published datapoints!"); return true; },
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
            warn!("Folder {} does not exist!", folder.display());
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                let filename: String = String::from(entry.path().to_str().unwrap());
                if self.get_extension_from_filename(&filename) == Some("json") {
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
                }
            }
        }
    }
    fn upload_activity_details(&mut self) {
        let base_path = String::from(&self.influx_config.file_base_path);
        let folder = Path::new(&base_path).join("activities");
        if !folder.exists() {
            warn!("Folder {} does not exist!", folder.display());
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                let filename: String = String::from(entry.path().to_str().unwrap());
                if self.get_extension_from_filename(&filename) == Some("fit") {
                    // we could use the below mapping to filter out fields for certain record kinds,
                    // but for now we'll scrape ALL valid fields and upload to DB. 
                    // let msp_field_mapping: HashMap<&str, HashSet<&str>> = msg_type_map::get_map();
                    let id = self.get_activity_id_from_filename(&filename);
                    self.parse_fit_file(&filename, "activity", Some(vec![("id".to_string(), id)]));
                }
            }
        }
    }

    fn get_activity_id_from_filename<'a>(&self, filename: &'a str) -> String {
        let re = Regex::new(r".*[\/|\\](\d+)_ACTIVITY\.fit").unwrap();
        for (_, [id]) in re.captures_iter(filename).map(|c| c.extract()) {
            return String::from(id);
        }
        error!("====================================================");
        panic!("Unable to parse activity id in filename: {}", filename);
    }

    fn get_monitoring_metric_from_filename<'a>(&self, filename: &'a str) -> String {
        let re = Regex::new(r".*[\/|\\]\d*_(.*)\.fit").unwrap();
        for (_, [metric]) in re.captures_iter(filename).map(|c| c.extract()) {
            return String::from(metric);
        }
        error!("====================================================");
        panic!("Unable to parse monitoring metrics in filename: {}", filename);
    }

    fn upload_sleep(&mut self) {
        let base_path = String::from(&self.influx_config.file_base_path);
        let folder = Path::new(&base_path).join("sleep");
        if !folder.exists() {
            warn!("Folder {} does not exist!", folder.display());
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                match File::open(entry.path()) {
                    Ok(_file) => {
                        // let reader = BufReader::new(file);
                        // let sleep: HashMap<String, serde_json::Value> = serde_json::from_reader(reader).unwrap();
                        
                        // let restless_moments = json!(sleep["sleepRestlessMoments"]);
                        // let sleep_levels = json!(sleep["sleepLevels"]);
                        // let hrv = json!(sleep["hrv"]);
                        // let sleep_stress = json!(sleep["sleepStress"]);
                        // let sleep_movement = json!(sleep["sleepMovement"]);
                    }, Err(e) => { error!("Unable to open file: {}, error: {:?}", entry.path().to_str().unwrap(), e) }
                }
            }
        }
    }
    fn upload_heart_rate_data(&mut self) {
        let base_path = String::from(&self.influx_config.file_base_path);
        let folder = Path::new(&base_path).join("heartrate");
        if !folder.exists() {
            warn!("Folder {} does not exist!", folder.display());
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                println!("Currently unable to parse summary json. File: {:?}", entry.path());
            }
        }
    }
    fn upload_weight_data(&mut self) {
        let base_path = String::from(&self.influx_config.file_base_path);
        let folder = Path::new(&base_path).join("weight");
        if !folder.exists() {
            warn!("Folder {} does not exist!", folder.display());
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                println!("Currently unable to parse summary json. File: {:?}", entry.path());
            }
        }
    }
    fn upload_summary_data(&mut self) {
        let base_path = String::from(&self.influx_config.file_base_path);
        let folder = Path::new(&base_path).join("day_summary");
        if !folder.exists() {
            warn!("Folder {} does not exist!", folder.display());
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                println!("Currently unable to parse summary json. File: {:?}", entry.path());
            }
        }
    }

    fn upload_monitoring(&mut self) {
        let base_path = String::from(&self.influx_config.file_base_path);
        let folder = Path::new(&base_path).join("monitoring");
        if !folder.exists() {
            warn!("Folder {} does not exist!", folder.display());
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                let filename: String = String::from(entry.path().to_str().unwrap());
                if self.get_extension_from_filename(&filename) == Some("fit") {
                    // we could use the below mapping to filter out fields for certain record kinds,
                    // but for now we'll scrape ALL valid fields and upload to DB. 
                    // let msp_field_mapping: HashMap<&str, HashSet<&str>> = msg_type_map::get_monitoring_map();
                    let monitoring_metric = self.get_monitoring_metric_from_filename(&filename);
                    self.parse_fit_file(&filename, "monitoring", Some(vec![("metric".to_string(), monitoring_metric)]));
                }
            }
        }
    }

    #[allow(dead_code)]
    fn examine_fit_file_records(&self, filename: &str){
        // use this to print all fields in all records in a fit file. just prints them to screen.
        let mut fp = File::open(filename).unwrap();
        let mut record_map: HashMap<String, HashSet<String>> = HashMap::new();

        for record in fitparser::from_reader(&mut fp).unwrap() {
            let kind: &str = &record.kind().to_string();

            match record_map.get_mut(kind) {
                Some(set) => {
                    for field in record.fields() {
                        set.insert(String::from(field.name()));
                    }
                }, None => {
                    let mut set: HashSet<String> = HashSet::new();
                    for field in record.fields() {
                        set.insert(String::from(field.name()));
                    }
                    record_map.insert(kind.to_string(), set);
                }
            }
        }

        for (rec_type, field_names) in record_map { println!("{}: {:?}", rec_type, field_names); }
    }

    fn parse_fit_file(&mut self, filename: &str, measurement: &str, tags: Option<Vec<(String, String)>>){
        let mut fp = File::open(filename).unwrap();
        let mut datapoints: Vec<DataPoint> = Vec::new();
        let mut activity: Option<String> = None;
        let records_to_include: Vec<String> = serde_json::from_value(self.influx_config.records_to_include.clone()).unwrap();
        let mut last_timestamp: HashMap<String, i64> = HashMap::new();

        for record in fitparser::from_reader(&mut fp).unwrap() {
            let kind: &str = &record.kind().to_string();

            if kind == "sport" {
                for field in record.into_vec() {
                    if field.name() == "name" {
                        activity = Some(field.value().to_string());
                        break;
                    }
                }
                continue;
            }

            // ignore this entire data point if the record isn't on 'the list'
            if !records_to_include.contains(&kind.to_string()) { continue; }

            let mut data = DataPoint::builder(measurement);
            if let Some(ref t) = tags { for (tag, value) in t { data = data.tag(tag, value); }}
            if let Some(ref activity_name) = activity { data = data.tag("activity", activity_name); }

            for field in record.into_vec() {
                // grab the timestamp.
                if field.name() == "timestamp" {
                    match DateTime::parse_from_str(&field.value().to_string().replace('"', ""), GARMIN_FIT_DATE_FORMAT){
                        Ok(ts) => { 
                            data = data.timestamp(ts.timestamp_nanos_opt().unwrap()); 
                            last_timestamp.insert(kind.to_string(), ts.timestamp());
                            // info!("Found timestamp: {} for record type: {}", ts.timestamp(), kind);
                        }, Err(e) => { 
                            error!("Unable to parse timestamp from 'timestamp' field value: {} in record type {}. Error: {}", &field.value(), kind, e);
                            break;
                        }
                    }
                // for 'monitoring' records, 'timestamp_16' represents offset from last epoch timestamp
                } else if field.name() == "timestamp_16" {
                    let timestamp_16 = field.value().to_string().parse::<i64>().unwrap();
                    if let Some(dt) = last_timestamp.get(&kind.to_string()) {
                        // dt is unix epoch seconds, in GMT - convert to garmin epoch
                        let mut garmin_date = *dt - GARMIN_EPOCH_OFFSET;

                        // increase by difference of lower 2 bytes of timestamp
                        garmin_date += (timestamp_16 - ( garmin_date & 0xFFFF ) ) & 0xFFFF;

                        // convert back to unix epoch
                        garmin_date += GARMIN_EPOCH_OFFSET;
                        let metric_date = NaiveDateTime::from_timestamp_opt(garmin_date, 0).unwrap();
                        data = data.timestamp(metric_date.timestamp_nanos_opt().unwrap());
                    }
                // some records have fields like 'unknown_field_X' - ignore those.
                // some records have another field called 'local_timestamp' - just ignore those too.
                } else if !field.name().contains("unknown") && !field.name().contains("timestamp") {
                    if let Ok(value) = field.value().to_string().parse::<f64>() {
                        data = data.field(String::from(field.name()), value);
                    }
                }
            }
            match data.build() {
                Ok(datapoint) => { datapoints.push(datapoint); },
                Err(e) => { warn!("Unable to build datapoint for record {}, error: {}", kind, e); }
            }
        }

        self.write_data(datapoints);
    }
}