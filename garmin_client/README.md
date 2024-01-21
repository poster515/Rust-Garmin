Example Usage:

```ignore
use std::collections::HashMap;
use garmin_client::GarminClient;
use chrono::NaiveDateTime;

fn main() {
    let args: Vec<String> = env::args().collect();

    // first need to login (hopefully that's obvious)
    let mut client = GarminClient::new();
    client.login(args[1], args[2]);

    // get endpoint for service
    let endpoint = "weight-service/weight/dateRange";

    let weight_date = "2023-01-01 00:00:00";
    let datetime = NaiveDateTime::parse_from_str(&weight_date, "%Y-%m-%d %H:%M:%S").unwrap();

    let params = HashMap::from([
        ("startDate", "2023-01-01"),
        ("endDate", "2023-01-01"),
        ("_", datetime.timestamp_millis().as_str())
    ]);

    let is_json_result = true;
    let filename = "weight.json";
    client.api_request(endpoint, Some(params), is_json_result, filename);
}
```