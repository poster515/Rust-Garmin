Basic Usage:

```ignore

use std::{env, path::Path};
use config::{Config, File, FileFormat};
use getopts::Matches;

fn main() {
    let config = Config::builder().add_source(File::new(cwd.join("config").join("garmin_config.json").to_str().unwrap(), FileFormat::Json)).build().unwrap();

    let mut download_manager = DownloadManager::new(config, matches.clone());
    download_manager.login();
    download_manager.download_all();
}
```