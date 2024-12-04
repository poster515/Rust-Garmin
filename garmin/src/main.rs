use anyhow::Error;
use log::{error, info};
use std::{env, path::Path};

use config::{Config, File, FileFormat};

use getopts::{Matches, Options};

use garmin_download::DownloadManager;
use influx_upload::UploadManager;

fn build_options() -> Options {
    // the presence of any of these options automatically enables
    // the download of the associated data
    let mut options = Options::new();
    options.optopt(
        "u",
        "summary_date",
        "download date for summary data",
        "use YYY-MM-DD format",
    );

    options.optopt(
        "w",
        "weight_date",
        "download date for weight data",
        "use YYY-MM-DD format",
    );

    options.optopt(
        "s",
        "sleep_date",
        "download date for sleep data",
        "use YYY-MM-DD format",
    );

    options.optopt(
        "r",
        "resting_heart_date",
        "download date for resting heart rate data",
        "use YYY-MM-DD format",
    );

    options.optopt(
        "o",
        "hydration_date",
        "download date for hydration data",
        "use YYY-MM-DD format",
    );

    options.optopt(
        "m",
        "monitor_date",
        "download date for monitoring data",
        "use YYY-MM-DD format",
    );

    options.optopt(
        "a",
        "activity_date",
        "download date for activity data",
        "use YYY-MM-DD format",
    );

    options.optopt(
        "e",
        "examine_file",
        "examines and prints records/field info for FIT file",
        "should be relative to file_base_path config",
    );

    options.optopt(
        "d",
        "download_activity",
        "ID of activity to download",
        "saves FIT and json files in <file_base_path>/activities",
    );

    options.optflag("", "print_activity_ids", "print all known activity IDs");

    options.optflag("h", "help", "print this help menu");

    options.optflag("", "disable_download", "ignores data download entirely");

    options.optflag("", "disable_upload", "ignores data upload entirely");

    options
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // takes various command line args, runs download once and exits
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let options = build_options();
    let matches: Matches = match options.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            panic!("{}", f.to_string())
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, options);
        return Ok(());
    }

    // use handle to change logger configuration at runtime.
    // example use cases: https://crates.io/crates/log4rs
    let file_path = env::current_exe().unwrap();
    let cwd: Box<Path> = env::current_dir().unwrap().as_path().into();

    println!("Current executable path: {:?}", file_path);

    let handle = log4rs::init_file(cwd.join("config").join("log4rs.yml"), Default::default());
    match handle {
        Ok(()) => {
            info!("Successfully loaded log config!");
        }
        Err(error) => {
            println!("Error loading log config: {:}", error);
            return Err(error);
        }
    }

    let handle = Config::builder()
        .add_source(File::new(
            cwd.join("config")
                .join("garmin_config.json")
                .to_str()
                .unwrap(),
            FileFormat::Json,
        ))
        .build();
    match handle {
        Ok(config) => {
            info!("Successfully loaded garmin config! Executing any configured downloads...");

            // login and download all configured stats
            let mut download_manager = DownloadManager::new(config, Some(matches.clone()));
            if matches.opt_present("disable_download") {
                info!("Not downloading any garmin data");
            } else {
                download_manager.login().await;
                download_manager.download_all().await;
            }

            if let Ok(Some(id)) = matches.opt_get::<String>("d") {
                info!("Attempting to download activity ID {}...", id);
                download_manager.login().await; // should be able to call this twice - client looks for session file
                download_manager
                    .get_activity_info(id.to_string().parse::<u64>().unwrap())
                    .await;
                download_manager
                    .get_activity_details(id.to_string().parse::<u64>().unwrap())
                    .await;
            }
        }
        Err(error) => {
            error!("Error loading garmin config: {:}", error);
            return Err(Into::into(error));
        }
    }

    // create config for use with uploader
    let handle = Config::builder()
        .add_source(File::new(
            cwd.join("config")
                .join("influxdb_config.json")
                .to_str()
                .unwrap(),
            FileFormat::Json,
        ))
        .build();
    match handle {
        Ok(config) => {
            info!("Successfully loaded influx config!");

            // spin up influx publisher and publish data
            let mut upload_manager = UploadManager::new(config);
            if matches.opt_present("disable_upload") {
                info!("Not uploading any garmin data");
            } else {
                upload_manager.upload_all().await;
            }

            if let Ok(Some(filename)) = matches.opt_get::<String>("e") {
                upload_manager.examine_fit_file_records(&filename);
            }

            if matches.opt_present("print_activity_ids") {
                // TODO: complete this function
                // let _ = upload_manager.get_uploaded_activity_ids();
            }
        }
        Err(error) => {
            error!("Error loading influxdb config: {:}", error);
            return Err(Into::into(error));
        }
    }

    Ok(())
}
