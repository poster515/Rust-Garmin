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

- If 'download_today_data' is true, only activities that started between midnight today (using local TZ) to midnight tomorrow morning will be saved.
- If 'save_regardless_of_date' is false, only activities from midnight on activity_start_date to midnight the next day will be saved.
- Else, the activity is by default saved to file.

Saving activities based on date is hard since there is no endpoint (to my knowledge) that searches for activities by date. You can download the N activities from activity_start_date. One option to download summaries for a large number activities, whose dates can be checked for correctness. One other future option might be to save activity summaries to influx and then directly expose the garmin_client to save activities by ID, but this is not currently implemented.

### Daily Usage (e.g., cron job)
I would recommend choosing a reasonable value (e.g., 10) to fetch info for activities, unles you think you'll be saving more than activities in one day, in which case you're crazy. Sample configs for daily download of yesterday's data:
- 'download_today_data': false
- 'save_regardless_of_date': false
- enable the stats you want

### Historical Download (e.g., bulk download)
If you wanted to, say, download a large number of historical activities, set the dates in the config file and an appropriate number of days' to download. Let's say you wanted to download the year's worth of *monitoring* data (heart rate, respiration rate, etc) from 2023. Set the following: 
- 'download_today_data': false
- 'monitor_start_date': '2023-01-01'
- disable all stats except 'monitoring'
- 'num_days_from_start_date': 365

One known limitation with the session management is that you can only request activity summaries ONCE per session token, and Garmin will lock you out for a few hours if you repeatedly abuse their OAuth2.0 architecture by constantly requesting new tokens. It is because of this that the 'num_days_from_start_date' config was added. If you need to download more monitoring data, there's a good chance you'll have to delete the .garmin_session.json file first.