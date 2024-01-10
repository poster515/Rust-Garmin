use std::env;
use log::{error, info, warn, debug};
use anyhow::Error;

use config::{Config, File, FileFormat};
// use float_cmp::ApproxEqUlps;

use download::{DownloadManager, DownloadTraits};

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
            let login_result = _download_manager.login();
            match login_result {
                Ok(success) => {
                    if success {
                        info!("Successfully logged in!");
                    } else {
                        warn!("Unable to login!");
                    }
                },
                Err(_) => {
                    println!("Error logging in :(")
                }
            }
        },
        Err(error) => {
            error!("Error loading log config: {:}", error);
            return Err(Into::into(error))
        }
    }

    Ok(())

}
