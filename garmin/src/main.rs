use std::env;
use log::{error, info};
use anyhow::Error;

use config::{Config, File, FileFormat};
// use float_cmp::ApproxEqUlps;

use download::{DownloadManager, DownloadTraits};

#[allow(unused_must_use)]
fn main() -> Result<(), Error> {
    // takes various command line args, runs download once and exits

    // use handle to change logger configuration at runtime.
    // example use cases: https://crates.io/crates/log4rs
    let file_path = env::current_exe().unwrap();
    println!("Current executable path: {:?}", file_path);

    let _handle = log4rs::init_file("/home/jpost/Documents/Rust-Garmin/log4rs.yml", Default::default());
    match _handle {
        Ok(()) => {
            info!("Successfully loaded log config!");
        },
        Err(error) => {
            println!("Error loading log config: {:}", error);
            return Err(error)
        }
    }

    // create config for use with downloader
    let _handle = Config::builder().add_source(File::new("/home/jpost/Documents/Rust-Garmin/garmin_config.json", FileFormat::Json)).build();
    match _handle {
        Ok(config) => {
            info!("Successfully loaded garmin config!");

            let mut _download_manager = DownloadManager::new(config);
            _download_manager.login();
            _download_manager.get_activity_types();
        },
        Err(error) => {
            error!("Error loading log config: {:}", error);
            return Err(Into::into(error))
        }
    }

    Ok(())

}
