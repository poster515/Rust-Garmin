## GarminGearBox
![Rust Build](https://github.com/github/docs/actions/workflows/rust.yml/badge.svg)
This repo contains a complete OAuth2.0 client capable of obtaining various garmin data from specified user account. Main files include:

### log4rs.yml
Use this file to dictate the logging behavior.

### garmin_config.json
Various dates/stats you want to download. Right now only the dates are used.

### Intended Use Case
This library is intended to provide cron-like downloads on a daily basis, although by editing main.rs you can use it as a simple command line utility. Right now data will save to log4rs log and not necssarily to file; this is still a WIP and not fully thought out yet.
