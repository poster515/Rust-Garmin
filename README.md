## GarminGearBox
![Rust Build](https://github.com/poster515/Rust-Garmin/actions/workflows/rust.yml/badge.svg?branch=main)

This repo contains a complete OAuth2.0 client capable of obtaining various garmin data from specified user account. Main files include:

### log4rs.yml
Use this file to dictate the logging behavior.

### garmin_config.json
Various dates/stats you want to download. Right now only the dates are used.

### Intended Use Case
This library is intended to provide cron-like downloads on a daily basis, although by editing main.rs you can use it as a simple command line utility. The app is generally configured to query and save those data specified in the garmin_config.json file. Options should generally be pretty obvious, an earnest attempt is made to make filenames as unique but intuitive as possible. For example we could have used UUIDs but that has a filename length consideration, as well as providing no immediately obvious significance.
