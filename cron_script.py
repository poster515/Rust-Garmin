import os
from datetime import date
from datetime import timedelta
import subprocess
import datetime
import json

if __name__ == '__main__':
    # run this script at 5am to download/upload previous day of data.

    # need to set 'download_today_data' to false in garmin_config.json!

    # First go through and prune the non-*.zip output files so we don't
    # parse them the next day.
    config_file = os.path.join(os.getcwd(), "config", "influxdb_config.json")
    with open(config_file, "r") as file:
        config = json.load(file)

    # remove files older than yesterday - allows us to keep yesterday's files for a whole day.
    output_folder = config["file_base_path"]
    tz = datetime.timezone(-timedelta(hours=5))
    yesterday = datetime.datetime.now(tz) - timedelta(days=1)
    print(f"Attempting to prune non-*zip files in output directory {output_folder}")
    for root, dirs, files in os.walk(output_folder):
        for name in files:
            filename = os.path.join(root, name)
            timestamp = os.path.getctime(filename)
            creation_date = datetime.datetime.fromtimestamp(timestamp, tz=tz)

            # only delete fit files for now since we're keeping the zips
            _, ext = os.path.splitext(filename)
            if creation_date < yesterday and ext in config["files_to_prune"]:
                print(f"Pruning old file {filename}, created {timestamp}")
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
    print(f"Executing command:\n\n{" ".join(exe)}\n")
    output = subprocess.run(exe, capture_output=True)
    print(output.stdout)
