Basic Usage:

```ignore

use std::{env, path::Path};
use config::{Config, File, FileFormat};
use getopts::Matches;

fn main() {
    let args: Vec<String> = env::args().collect();

    // see https://github.com/poster515/Rust-Garmin/blob/main/garmin/src/main.rs for options building
    let options = Options::new();
    let matches: Matches = match options.parse(&args[1..]).unwrap();

    let config = Config::builder().add_source(File::new(cwd.join("config").join("garmin_config.json").to_str().unwrap(), FileFormat::Json)).build().unwrap();

    let mut download_manager = DownloadManager::new(config, matches);
    download_manager.login();
    download_manager.download_all();
}
```