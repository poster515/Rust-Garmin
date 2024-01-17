## GarminGearBox
![Rust Build](https://github.com/poster515/Rust-Garmin/actions/workflows/rust.yml/badge.svg?branch=main)

This repo contains a complete OAuth2.0 client capable of obtaining various garmin data from specified user account. Main files include:

### log4rs.yml
Use this file to dictate the logging behavior.

### garmin_config.json
Various dates/stats you want to download. Can generally be overridden via command line args.

### influxdb_config.json
Influxdb login credentials.

### Intended Use Case
This library is intended to provide cron-like downloads on a daily basis, although by editing main.rs you can use it as a simple command line utility. The app is generally configured to query and save those data specified in the garmin_config.json file. Options should generally be pretty obvious, an earnest attempt is made to make filenames as unique but intuitive as possible. For example we could have used UUIDs but that has a filename length consideration, as well as providing no immediately obvious significance.

#### Download Behavior
All downloads can be configured via the config/garmin_config.json file. Here, various bools can be set to specify what data to download from which date, and to which root output folder. The data dates for each activity can be explicitly overridden via command line argument, and if specified as an input argument will download that data for that date regardless of json config.

All downloads are placed in subfolders within the file_base_path (e.g., "sleep", "heartrate"). Downloads will likely fail to save files until those subfolders are made.

Downloads can be disabled entirely by passing --disable_downloads as an input argument.

#### Upload Behavior
Right now the intended target is an influxdb server, although this repo should be expanded to target more destinations. Configure the influxDB client via config/influxdb_config.json.

Uploads can be disabled entirely by passing --disable_uploads as an input argument.