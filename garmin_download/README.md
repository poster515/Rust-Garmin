## Basic Usage:

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
    -a, --activity_date use YYY-MM-DD format
                        download date for activity data
```

By passing a Some(matches) object from the getopts crate you can specify these overrides. See ../garmin/src/main.rs for an example.

### Activity Gotchyas
There is a config 'num_activities_to_download' which requests summaries for the last N activities, including start times and activity IDs. From here, another API is called to actually retrieve detailed summaries for each activity.

Use the following flow to understand how activity details are actually saved:

-If 'download_today_data' is true, only activities that started between midnight today (using local TZ) to midnight tomorrow morning will be saved. 
-If 'save_regardless_of_date' is false, only activities from midnight on activity_start_date to midnight the next day will be saved.
-Else, the activity is by default saved to file.

If you are using this script programmatically (e.g., daily), would recommend choosing a reasonable value (e.g., 10) to fetch info for activities. If you wanted to, say, download a large number of historical activities, set 'download_today_data' to false, 'save_regardless_of_date' to true, and 'num_activities_to_download' to a large number (1000 or so? haven't stress tested that api personally). This would download summary data (FIT files) for the last 1000 activities.

One known issue with the session management is that you can only request activity summaries ONCE per session token, and Garmin will lock you out for a few hours if you repeatedly abuse their OAuth2.0 architecture by constantly requesting new tokens.