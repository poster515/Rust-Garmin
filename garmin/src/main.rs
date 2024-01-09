
use log::{error, info, warn, debug};

use config::{Config, File, FileFormat};
// use float_cmp::ApproxEqUlps;

use download::DownloadManager;

fn main() {
    // takes various command line args, runs download once and exits

    // use handle to change logger configuration at runtime.
    // example use cases: https://crates.io/crates/log4rs
    let _handle = log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    // create config for use with downloader
    let config = Config::builder().add_source(File::new("tests/Settings", FileFormat::Json)).build().unwrap();

    let download_manager = DownloadManager::new(config);
    
    debug!("Mary has a little lamb");
    error!("{}", "Its fleece was white as snow");
    info!("{:?}", "And every where that Mary went");
    warn!("{:#?}", "The lamb was sure to go");

}
