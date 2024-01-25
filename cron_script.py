import os
from datetime import date
from datetime import timedelta
import subprocess

if __name__ == '__main__':
    # run this script at 5am to download/upload previous day of data.

    # need to set 'download_today_data' to false in config!

    # set the dates for yesterday so we get a whole days of data
    exe = [os.path.join(os.getcwd(), "target", "debug", "garmin")]
    yesterday = date.today() - timedelta(days = 1)
    options = {
        "--summary_date" : f"{yesterday}",
        "--weight_date" : f"{yesterday}",
        "--sleep_date" : f"{yesterday}",
        "--resting_heart_date" : f"{yesterday}",
        "--monitor_date" : f"{yesterday}",
        "--hydration_date" : f"{yesterday}"
    }

    for metric, dt in options.items():
        exe.append(metric)
        exe.append(dt)
    print(f"Executing command:\n\n{" ".join(exe)}\n")
    output = subprocess.run(exe, capture_output=True)
    print(output.stdout)
