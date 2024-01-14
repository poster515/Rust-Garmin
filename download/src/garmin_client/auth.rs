
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use futures::executor;
use log::{error, debug, warn};
use chrono::{DateTime, Local};

use reqwest;
use reqwest::header::HeaderMap;

use reqwest_oauth1;
use reqwest_oauth1::{OAuthClientProvider, TokenReaderFuture, TokenResponse};

use serde::Deserialize;

#[derive(Default, Deserialize)]
struct ConsumerInfo {
    consumer_key: String,
    consumer_secret: String
}

#[derive(Default)]
pub struct OAuth1Token {
    oauth_token: String,
    oauth_token_secret: String,
    mfa_token: String,
    mfa_expiration_timestamp: DateTime<Local>,
    domain: String
}

#[derive(Default)]
pub struct OAuth2Token {
    scope: String,
    jti: String,
    token_type: String,
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    expires_at: u64,
    refresh_token_expires_in: u64,
    refresh_token_expires_at: u64
}

impl OAuth2Token {
    fn expired(&self) -> bool {
        return self.expires_at < SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    fn refresh_expired(&self) -> bool {
        return self.refresh_token_expires_at < SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    fn to_string(&self) -> String {
        format!("{} {}", self.token_type, self.access_token)
    }
}

pub struct GarminOAuth1Session {
    OAUTH_CONSUMER_URL: String,
    OAUTH_CONSUMER: HashMap<String, String>,
    USER_AGENT: Vec<String>,
    consumer_info: ConsumerInfo,
    oauth1_token: OAuth1Token,
    oauth1_client: reqwest::Client
}

impl GarminOAuth1Session {
    pub fn new () -> GarminOAuth1Session {
        GarminOAuth1Session {
            OAUTH_CONSUMER_URL: String::from("https://thegarth.s3.amazonaws.com/oauth_consumer.json"),
            OAUTH_CONSUMER: HashMap::new(),
            USER_AGENT: vec!["User-Agent".to_owned(), "com.garmin.android.apps.connectmobile".to_owned()],
            consumer_info: Default::default(),
            oauth1_token: Default::default(),
            oauth1_client: reqwest::Client::new()
        }
    }

    pub fn get_oauth1_token(&mut self, ticket: &str) -> String {
        self.consumer_info = reqwest::blocking::get(&self.OAUTH_CONSUMER_URL)
            .unwrap()
            .json::<ConsumerInfo>()
            .unwrap();

        let secrets = reqwest_oauth1::Secrets::new(
            &self.consumer_info.consumer_key, 
            &self.consumer_info.consumer_secret
        );

        let mut endpoint_reqtoken: String = String::from("https://connectapi.garmin.com/oauth-service/oauth/preauthorized");
        endpoint_reqtoken.push_str("?ticket=");
        endpoint_reqtoken.push_str(ticket);
        endpoint_reqtoken.push_str("?login-url=https://sso.garmin.com/sso/embed&accepts-mfa-tokens=true");

        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", "com.garmin.android.apps.connectmobile".parse().unwrap());
        
        let client = reqwest::Client::new();
        let response = client
            .oauth1(secrets)
            .post(&endpoint_reqtoken)
            .headers(headers)
            .query(&[("oauth_callback", "oob")])
            .send()
            .parse_oauth_token();

        let resp = executor::block_on(response).unwrap();

        println!(
            "your token and secret is: \n token: {}\n secret: {}",
            resp.oauth_token, resp.oauth_token_secret
        );

        // let response_html = response.text().unwrap();

        // debug!("Got the following oauth response: {}", response_html);

        resp.oauth_token

        // TODO: get adapters/proxies/vverify fields from original reqwest client
        /*
        if parent is not None:
            self.mount("https://", parent.adapters["https://"])
            self.proxies = parent.proxies
            self.verify = parent.verify
         */
    }
}

        