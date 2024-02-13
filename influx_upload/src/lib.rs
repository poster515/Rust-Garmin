
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
use async_recursion::async_recursion;

mod influxdb_structs;
use influxdb_structs::InfluxDbConfig;

mod msg_type_map;

// actually contains a T but we'll replace that with a 
// space since the DateTime mod can't decode that for
// some reason.
const GARMIN_JSON_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";
const GARMIN_FIT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S %z";
const GARMIN_EPOCH_OFFSET: i64 = 631065600;
const GARMIN_POSITION_FACTOR: f64 = 11930465.0;

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

    pub async fn upload_all(&mut self) {
        // first get set of all previously uploaded activity IDs
        let previous_activity_ids = self.get_activity_ids().await;

        if self.influx_config.upload_json_files {
            self.upload_activity_info(&previous_activity_ids).await;
            self.upload_heart_rate_data();
            self.upload_summary_data();
            self.upload_weight_data();
            self.upload_sleep();
        } else {
            info!("Ignoring JSON file uploads");
        }

        if self.influx_config.upload_fit_files {
            self.upload_monitoring().await;
            self.upload_activity_details(&previous_activity_ids).await;
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

    #[async_recursion]
    async fn get_activity_ids(&mut self) -> Vec<String>{
        match self.influx_client.as_ref() {
            Some(client) => {
                let ids = client.list_measurement_tag_values(
                    &self.influx_config.bucket,
                    "activity_details",
                    "activityId",
                    None,
                    None
                )
                .await
                .unwrap();
                
                info!("Got {} previous activity ids", ids.len());
                ids
            }, None => {
                warn!("InfluxDb client not configured yet!");
                if !self.build_client() { return vec![]; }
                return self.get_activity_ids().await;
            }
        }
    }

    #[async_recursion]
    async fn write_data(&mut self, data: Vec<DataPoint>) -> bool {
        match self.influx_client.as_ref() {
            Some(client) => {
                let num = data.len();

                match client.write(&self.influx_config.bucket, stream::iter(data)).await {
                    Ok(_) => { info!("Published {} datapoints!", num); return true; },
                    Err(e) => { error!("Unable to write data point(s): {:?}", e); return false; }
                }
            }, None => {
                warn!("InfluxDb client not configured yet!");
                if !self.build_client() { return false; }
                return self.write_data(data).await;
            }
        }
    }

    fn get_extension_from_filename<'a>(&'a self, filename: &'a str) -> Option<&str> {
        Path::new(filename).extension().and_then(OsStr::to_str)
    }

    fn search_for_float(&self, data: &serde_json::Value, key: &str) -> Option<f64> {
        match data.get(key) {
            Some(value) => { value.as_f64()
            }, None => { None }
        }
    }

    fn search_for_i64(&self, data: &serde_json::Value, key: &str) -> Option<i64> {
        match data.get(key) {
            Some(value) => { value.as_i64()
            }, None => { None }
        }
    }

    async fn upload_activity_info(&mut self, prev_ids: &Vec<String>) {
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
                            let activity_id = &activity["activityId"].to_string().replace('"', "");

                            let timestamp = self.garmin_ts_to_nanos_since_epoch(activity_data["startTimeLocal"].as_str().unwrap());

                            if prev_ids.contains(&activity_id){
                                if !self.influx_config.override_activites {
                                    info!("Id {} already exists, not overriding...", activity_id);
                                    continue;
                                }
                            }

                            let mut data = DataPoint::builder("activity_summary")
                                .tag("activityName",    activity["activityTypeDTO"]["typeKey"].to_string().replace('"', ""))
                                .tag("activityId",      activity_id)
                                .field("name",            activity["activityName"].to_string().replace('"', ""));

                            if let Some(float) = self.search_for_float(activity_data, "activityTrainingLoad") { data = data.field("activityTrainingLoad", float); }
                            if let Some(float) = self.search_for_float(activity_data, "anaerobicTrainingEffect") { data = data.field("anaerobicTrainingEffect", float); }
                            if let Some(float) = self.search_for_float(activity_data, "averageHR") { data = data.field("averageHR", float); }
                            if let Some(float) = self.search_for_float(activity_data, "averageSpeed") { data = data.field("averageSpeed", float); }
                            if let Some(float) = self.search_for_float(activity_data, "avgRespirationRate") { data = data.field("avgRespirationRate", float); }
                            if let Some(float) = self.search_for_float(activity_data, "bmrCalories") { data = data.field("bmrCalories", float); }
                            if let Some(float) = self.search_for_float(activity_data, "calories") { data = data.field("calories", float); }
                            if let Some(float) = self.search_for_float(activity_data, "distance") { data = data.field("distance", float); }
                            if let Some(float) = self.search_for_float(activity_data, "duration") { data = data.field("duration", float); }
                            if let Some(float) = self.search_for_float(activity_data, "elapsedDuration") { data = data.field("elapsedDuration", float); }
                            if let Some(float) = self.search_for_float(activity_data, "maxHR") { data = data.field("maxHR", float); }
                            if let Some(float) = self.search_for_float(activity_data, "maxRespirationRate") { data = data.field("maxRespirationRate", float); }
                            if let Some(float) = self.search_for_float(activity_data, "minActivityLapDuration") { data = data.field("minActivityLapDuration", float); }
                            if let Some(float) = self.search_for_float(activity_data, "minRespirationRate") { data = data.field("minRespirationRate", float); }
                            if let Some(float) = self.search_for_float(activity_data, "movingDuration") { data = data.field("movingDuration", float); }
                            if let Some(float) = self.search_for_float(activity_data, "trainingEffect") { data = data.field("trainingEffect", float); }

                            if let Some(int) = self.search_for_i64(activity_data, "steps") { data = data.field("steps", int); }
                            if let Some(int) = self.search_for_i64(activity_data, "moderateIntensityMinutes") { data = data.field("moderateIntensityMinutes", int); }
                            if let Some(int) = self.search_for_i64(activity_data, "vigorousIntensityMinutes") { data = data.field("vigorousIntensityMinutes", int); }

                            self.write_data(vec![data.timestamp(timestamp).build().unwrap()]).await;

                        }, Err(e) => { error!("Failed to open file {:?}, error: {}", entry.path(), e); }
                    }
                }
            }
        }
    }

    async fn upload_activity_details(&mut self, prev_ids: &Vec<String>) {
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
                    let activity_id = self.get_activity_id_from_filename(&filename);
                    if prev_ids.contains(&activity_id){
                        if !self.influx_config.override_activites {
                            info!("Id {} already exists, not overriding...", activity_id);
                            continue;
                        }
                    }

                    self.parse_fit_file(&filename, "activity_details", Some(vec![("activityId".to_string(), activity_id)])).await;
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
                warn!("Currently unable to parse summary json. File: {:?}", entry.path());
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
                warn!("Currently unable to parse summary json. File: {:?}", entry.path());
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
                warn!("Currently unable to parse summary json. File: {:?}", entry.path());
            }
        }
    }

    async fn upload_monitoring(&mut self) {
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
                    self.parse_fit_file(&filename, "monitoring", Some(vec![("metric".to_string(), monitoring_metric)])).await;
                }
            }
        }
    }

    pub fn examine_fit_file_records(&self, filename: &str){
        // use this to print all fields in all records in a fit file. just prints them to screen.
        let mut fp = File::open(filename).unwrap();
        let mut record_map: HashMap<String, HashSet<String>> = HashMap::new();

        for record in fitparser::from_reader(&mut fp).unwrap() {
            let kind: &str = &record.kind().to_string();

            match record_map.get_mut(kind) {
                Some(set) => {
                    for field in record.fields() {
                        set.insert(String::from(field.name()).replace('"', ""));
                    }
                }, None => {
                    let mut set: HashSet<String> = HashSet::new();
                    for field in record.fields() {
                        set.insert(String::from(field.name()).replace('"', ""));
                    }
                    record_map.insert(kind.to_string(), set);
                }
            }
        }

        for (rec_type, field_names) in record_map { println!("{}: {:?}", rec_type, field_names); }
    }

    async fn parse_fit_file(&mut self, filename: &str, measurement: &str, tags: Option<Vec<(String, String)>>){
        let mut fp = File::open(filename).unwrap();
        let mut datapoints: Vec<DataPoint> = Vec::new();
        let records_to_include: Vec<String> = serde_json::from_value(self.influx_config.records_to_include.clone()).unwrap();
        let mut last_timestamp: HashMap<String, i64> = HashMap::new();

        for record in fitparser::from_reader(&mut fp).unwrap() {
            let kind: &str = &record.kind().to_string();

            // ignore this entire data point if the record isn't on 'the list'
            if !records_to_include.contains(&kind.to_string()) { continue; }

            let mut data = DataPoint::builder(measurement);
            if let Some(ref t) = tags { for (tag, value) in t { data = data.tag(tag.replace('"', ""), value.replace('"', "")); }}

            for field in record.into_vec() {
                // grab the timestamp.
                if field.name() == "timestamp" {
                    match DateTime::parse_from_str(&field.value().to_string().replace('"', ""), GARMIN_FIT_DATE_FORMAT){
                        Ok(ts) => { 
                            data = data.timestamp(ts.timestamp_nanos_opt().unwrap()); 
                            last_timestamp.insert(kind.to_string(), ts.timestamp());
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
                // garmin represents position data as 32 bit unsigned int, so we have to divide by representation 
                // range to get actual float.
                } else if field.name().contains("_lat") || field.name().contains("_long") {
                    if let Ok(value) = field.value().to_string().parse::<f64>() {
                        data = data.field(String::from(field.name()), value / GARMIN_POSITION_FACTOR);
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
                Err(_) => {}
            }
            
        }

        self.write_data(datapoints).await;
    }
}


#[cfg(test)]
mod tests {

    use config::{Config, File, FileFormat};
    use std::env::current_dir;
    use crate::UploadManager;

    #[test]
    fn timestamp_to_nanos_test() {
        let config = Config::builder().add_source(
            File::new(
                current_dir()
                    .unwrap()
                    .join("..")
                    .join("config")
                    .join("influxdb_config.json")
                    .to_str()
                    .unwrap(), 
                FileFormat::Json))
            .build()
            .unwrap();
        let um = UploadManager::new(config);
        let good_date = "2024-02-01 00:00:00.000";
        assert_eq!(um.garmin_ts_to_nanos_since_epoch(good_date), 1706745600000000000);
    }

    #[test]
    fn search_for_float_test() {
        let config = Config::builder().add_source(
            File::new(
                current_dir()
                    .unwrap()
                    .join("..")
                    .join("config")
                    .join("influxdb_config.json")
                    .to_str()
                    .unwrap(), 
                FileFormat::Json))
            .build()
            .unwrap();
        let data: serde_json::Value = serde_json::from_str("{ \"data\": 0.23432 }").unwrap();
        let key = "data";
        let um = UploadManager::new(config);
        assert_eq!(um.search_for_float(&data, key), Some(0.23432));
    }

    #[test]
    fn search_for_i64_test() {
        let config = Config::builder().add_source(
            File::new(
                current_dir()
                    .unwrap()
                    .join("..")
                    .join("config")
                    .join("influxdb_config.json")
                    .to_str()
                    .unwrap(), 
                FileFormat::Json))
            .build()
            .unwrap();
        let data: serde_json::Value = serde_json::from_str("{ \"data\": 1800 }").unwrap();
        let key = "data";
        let um = UploadManager::new(config);
        assert_eq!(um.search_for_i64(&data, key), Some(1800));
    }

    #[test]
    fn search_for_file_extension_test() {
        let config = Config::builder().add_source(
            File::new(
                current_dir()
                    .unwrap()
                    .join("..")
                    .join("config")
                    .join("influxdb_config.json")
                    .to_str()
                    .unwrap(), 
                FileFormat::Json))
            .build()
            .unwrap();
        let um = UploadManager::new(config);
        assert_eq!(um.get_extension_from_filename("test.json"), Some("json"));
    }
}