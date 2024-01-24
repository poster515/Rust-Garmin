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

    let mut download_manager = DownloadManager::new(config, Some(matches));
    download_manager.login();
    download_manager.download_all();
}
```

Note that 'download_data_today' in the config is a sort of universal override - it will ONLY download today's data for everything. Even if you specify a date override on the command line, that will be ignored if download_today_data is set to true. With that said, here are CLI overrides for various metrics with download_today_data set to false:
```
    -u, --summary_date use YYY-MM-DD format
                        download date for summary data
    -w, --weight_date use YYY-MM-DD format
                        download date for weight data
    -s, --sleep_date use YYY-MM-DD format
                        download date for sleep data
    -r, --resting_heart_date use YYY-MM-DD format
                        download date for resting heart rate data
    -m, --monitor_date use YYY-MM-DD format
                        download date for monitoring data
    -o, --hydration_date use YYY-MM-DD format
                        download date for hydration data
```

By passing a Some(matches) object from the getopts crate you can specify these overrides. See ../garmin/src/main.rs for an example.