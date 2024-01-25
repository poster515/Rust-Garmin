## GarminGearBox
![Rust Build](https://github.com/poster515/Rust-Garmin/actions/workflows/rust.yml/badge.svg?branch=main)

This repo contains libraries for downloading JSON and FIT file data from Garmin, parsing said files, and uploading data into an InfluxDb server. This is a side project that will likely always be a work in progress.

### Basic Configs

#### log4rs.yml
Use this file to dictate the logging behavior.

#### garmin_config.json
Various dates/stats you want to download. Can generally be overridden via command line args.

#### influxdb_config.json
Influxdb login credentials and basic upload behavior. No command line arg overrides yet.

### Intended Use Case
This library is intended to provide cron-like downloads on a daily basis, although by editing main.rs you can use it as a simple command line utility. The app is generally configured to query and save those data specified in the garmin_config.json file. Options should generally be pretty obvious, an earnest attempt is made to make filenames as unique but intuitive as possible. For example we could have used UUIDs but that has a filename length consideration, as well as providing no immediately obvious significance.

#### Garmin Download Behavior
All downloads can be configured via the config/garmin_config.json file. Here, various bools can be set to specify what data to download from which date, and to which root output folder. The data dates for each activity can be explicitly overridden via command line argument, and if specified as an input argument will download that data for that date regardless of json config.

All downloads are placed in subfolders within the file_base_path (e.g., "sleep", "heartrate"). *Downloads will likely fail to save files until those subfolders are made.*

Downloads can be disabled entirely by passing --disable_downloads as an input argument.

Note that 'download_data_today' is a sort of universal override - it will ONLY download today's data for everything. Even if you specify a date override on the command line, that will be ignored if download_today_data is set to true. With that said, here are CLI overrides for various metrics with download_today_data set to false:
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
    -a  --activity_date use YYY-MM-DD format
                        download date for activity data
```

#### Upload Behavior
Right now the intended target is an influxdb server, although this repo should be expanded to target more destinations. Configure the influxDB client via config/influxdb_config.json.

Uploads can be disabled entirely by passing --disable_uploads as an input argument.

### Influx DB Sample Setup
In this example, we'll set up a very basic influxDb 2.0 server, along with a grafana server for better visualizations and telegraf for any optional monitoring stats you want.

A few basics:
- everything will be in a docker container. If for some reason you want to run in a tmux/byobu session then you'll have to find equivalent examples online.
- the docker containers must share a docket network to communicate properly
- the docker containers will be mounted to a host volume so we can retrieve data from the host if needed. These can just as easily be docker volumes that you define separately.

#### Create Host Folder Structure
I made a directory structure as follows:
```
/etc/metrics
├── grafana
├── influxdb
│   └── data
│   └── config
└── telegraf
    └── telegraf.conf
```

The basic idea is to keep everything related to this project in the same folder: /etc/metrics. You must create the /data and /config subfolder in /influxdb if you want to see you data from the host. Currently I don't, but it feels like a good thing to have available.

I also chose to run a grafan server and telegraf server which is not required at all for this project. We'll configure these after influx is running. 

#### Create Docker Network
```
docker network create influxdb-telegraf-net
```

#### Create the Influx Docker Container
```
sudo docker run -d --rm --name=influxdb -p 8086:8086 -v /etc/metrics/influxdb:/var/lib/influxdb2       --net=influxdb-telegraf-net influxdb:2.0
```

#### Create the Grafana Docker Container (optional)
```
sudo docker run -d --rm -p 3000:3000 --name=grafana --volume /etc/metrics/grafana:/var/lib/grafana --net=influxdb-telegraf-net grafana/grafana-enterprise
```

#### Create the Telegraf Docker Container (optional)
```
sudo docker run -d --rm  --name=telegraf -v /etc/metrics/telegraf/telegraf.conf:/etc/telegraf/telegraf.conf --net=influxdb-telegraf-net telegraf
```

#### Usage Notes and Thoughts
I could have made a docker-compose.yml file. Fair. I had to do a fair bit of start, stopping, and removing influx docker containers to get my setup correct so this is the approach I took. Given the relative simplicity of the setup it may not warrant a docker compose file anyway.

Note the each docker container is spawned with ```-d --rm``` flags. These indicate that the docker daemon should 'detach' the container i.e., run it in the background and also to 'remove' the container once it stops. I did this because if you want to startup a new docker influx container, for example, docker will tell you that the named container is already in use.

Once the influxdb container is running, you'll need to create a bucket and a few API access tokens. I did this all using the web service running at ```<host IP address>:8086```, specifically 192.168.0.105:8086 for me. My server also uses a static IP so this will never (ideally) change. I then created a bucket called 'garmin' but you do you. Create tokens as specified in the official docs: https://docs.influxdata.com/influxdb/cloud/admin/tokens/create-token/. Create tokens for the following:
- rust upload client (read and write perms)
- grafana (only need read perms unless you're doing alerting or something)
- telegraf (read and write perms)

Now you should have everything to update your influxdb_config.json file:
- bucket name
- IP address
- organization
- API token

For almost all monitoring metrics it turns out that the FIT files contain everything you need - the JSON files downloaded are good to have as a reference but don't contain nearly as much info as the FIT files.

In influx you can configure telegraf via UI. Simply specify what things you want telegraf to monitor and the influx UI will generate a telegraf.conf for you. Copy and pastet that into the /etc/metrics/telegraf/telegraf.conf file from earlier, and insert your API token in the influxdb section.

Configuring grafana is also fairly straight forward. I configured mine using the FluxQL UI, using only the API token generated from the influx UI. Currently I have yet to do more with grafana but will hopefully update this doc if I do.

### Activity Gotchyas
There is a config 'num_activities_to_download' which requests summaries for the last N activities, including start times and activity IDs. From here, another API is called to actually retrieve detailed summaries for each activity.

Use the following flow to understand how activity details are actually saved:

-If 'download_today_data' is true, only activities that started between midnight today (using local TZ) to midnight tomorrow morning will be saved. 
-If 'save_regardless_of_date' is false, only activities from midnight on activity_start_date to midnight the next day will be saved.
-Else, the activity is by default saved to file.

If you are using this script programmatically (e.g., daily), would recommend choosing a reasonable value (e.g., 10) to fetch info for activities. If you wanted to, say, download a large number of historical activities, set 'download_today_data' to false, 'save_regardless_of_date' to true, and 'num_activities_to_download' to a large number (1000 or so? haven't stress tested that api personally). This would download summary data (FIT files) for the last 1000 activities.

One known issue with the session management is that you can only request activity summaries ONCE per session token, and Garmin will lock you out for a few hours if you repeatedly abuse their OAuth2.0 architecture by constantly requesting new tokens.