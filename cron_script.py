import os
import sys
from datetime import date
from datetime import timedelta
import subprocess
import datetime
import json
import time

if __name__ == '__main__':
    # run this script at 5am to download/upload previous day of data.

    # first argument needs to be the absolute path of this repo folder, otherwise it must
    # be executed from within the repo folder. This impacts cron for example, which executes
    # scripts from within the users home folder and messes up all the file paths for various 
    # config files.
    if len(sys.argv) > 1:
        os.chdir(sys.argv[1])

    # remove the session file in case the expiration date is more than 1 day
    try:
        with open(".garmin_session.json") as f:
            contents = json.load(f)
        if time.time() > int(contents['expires_at']):
            print("Removing state session file")
            os.remove(".garmin_session.json")
        else:
            print("Found valid session file!")
    except FileNotFoundError:
        pass

    # need to set 'download_today_data' to false in garmin_config.json!
    # also make sure that 'num_days_from_start_date' is set to correct 
    # number of days. If this is a daily job then set to 1.

    # First go through and prune the non-*.zip output files so we don't
    # parse them the next day.
    config_file = os.path.join(os.getcwd(), "config", "influxdb_config.json")
    with open(config_file, "r") as file:
        config = json.load(file)

    # remove files older than midnight - allows us to keep yesterday's files for a whole day.
    output_folder = config["file_base_path"]
    tz = datetime.timezone(-timedelta(hours=5))
    midnight = datetime.datetime.combine(date.today(), datetime.time(0, 0, 0, 0, tz), tzinfo=tz)
    print(f"Attempting to prune files in output directory {output_folder}")
    for root, dirs, files in os.walk(output_folder):
        for name in files:
            filename = os.path.join(root, name)
            timestamp = os.path.getctime(filename)
            creation_date = datetime.datetime.fromtimestamp(timestamp, tz=tz)

            _, ext = os.path.splitext(filename)
            if creation_date < midnight and ext in config["files_to_prune"]:
                print(f"Pruning old file {filename}, created {creation_date}")
                os.remove(filename)

    # set the dates for yesterday so we get a whole days of data
    exe = [os.path.join(os.getcwd(), "target", "debug", "garmin")]
    yesterday = date.today() - timedelta(days=1)
    options = {
        "--summary_date" : f"{yesterday}",
        "--weight_date" : f"{yesterday}",
        "--sleep_date" : f"{yesterday}",
        "--resting_heart_date" : f"{yesterday}",
        "--monitor_date" : f"{yesterday}",
        "--hydration_date" : f"{yesterday}",
        "--activity_date": f"{yesterday}",
    }

    for metric, dt in options.items():
        exe.append(metric)
        exe.append(dt)
    print(f"Executing command:\n\n{' '.join(exe)}\n")
    output = subprocess.run(exe, capture_output=True)
    print(output.stdout)
