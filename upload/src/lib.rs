
use config::Config;
use influxdb_structs::InfluxDbConfig;
use log::info;
use std::path::Path;
use influxdb::Client;

mod influxdb_structs;

// Class for downloading health data from Garmin Connect.
pub struct UploadManager {
    influx_config: InfluxDbConfig,
    influx_client: Client
}

impl UploadManager {
    pub fn new(config: Config) -> UploadManager {
        let mut um = UploadManager {
            influx_config: config.try_deserialize().unwrap(),
            influx_client: Client::new("dummy", "dummy")
        };
        let url: &str = &um.influx_config.url;
        let database: &str = &um.influx_config.database;
        let username: &str = &um.influx_config.username;
        let password: &str = &um.influx_config.password;
        um.influx_client = Client::new(url, database)
            .with_auth(username, password);

        info!("Configured influx client: \n{:?}", um.influx_client);
        um
    }

    pub fn upload_all(&mut self) {
        self.upload_activity_data();
        self.upload_heart_rate_data();
        self.upload_summary_data();
        self.upload_weight_data();
        self.upload_sleep();
    }

    fn upload_activity_data(&mut self) {
        let base_path = String::from(&self.influx_config.file_base_path);
        let folder = Path::new(&base_path).join("activities");
        if !folder.exists() {
            return;
        }
        for entry in folder.read_dir().expect(&format!("Could not open folder {:?} for reading", folder)) {
            if let Ok(entry) = entry {
                println!("{:?}", entry.path());
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
                println!("{:?}", entry.path());
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