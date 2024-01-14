
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

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
#[allow(dead_code)]
pub struct OAuth1Token {
    oauth_token: String,
    oauth_token_secret: String,
    mfa_token: String,
    mfa_expiration_timestamp: DateTime<Local>,
    domain: String
}

#[derive(Default)]
#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
pub struct GarminOAuth1Session {
    oauth_consumer_url: String,
    oauth_consumer: HashMap<String, String>,
    user_agent: Vec<String>,
    consumer_info: ConsumerInfo,
    oauth1_token: OAuth1Token,
    oauth1_client: reqwest::Client
}

impl GarminOAuth1Session {
    pub fn new () -> GarminOAuth1Session {
        GarminOAuth1Session {
            oauth_consumer_url: String::from("https://thegarth.s3.amazonaws.com/oauth_consumer.json"),
            oauth_consumer: HashMap::new(),
            user_agent: vec!["User-Agent".to_owned(), "com.garmin.android.apps.connectmobile".to_owned()],
            consumer_info: Default::default(),
            oauth1_token: Default::default(),
            oauth1_client: reqwest::Client::new()
        }
    }

    pub fn get_oauth1_token(&mut self, ticket: &str) -> Result<String, reqwest_oauth1::Error> {
        self.consumer_info = reqwest::blocking::get(&self.oauth_consumer_url)
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

        let rt = tokio::runtime::Runtime::new().unwrap();
        let future = rt.block_on({
            let response = client
                .oauth1(secrets)
                .post(&endpoint_reqtoken)
                .headers(headers)
                .query(&[("oauth_callback", "oob")])
                .send()
                .parse_oauth_token();
            response
        });

        let token: TokenResponse = future.unwrap();

        println!(
            "your token and secret is: \n token: {}\n secret: {}",
            token.oauth_token, token.oauth_token_secret
        );

        // let response_html = response.text().unwrap();

        // debug!("Got the following oauth response: {}", response_html);

        Ok(token.oauth_token)

        // TODO: get adapters/proxies/vverify fields from original reqwest client
        /*
        if parent is not None:
            self.mount("https://", parent.adapters["https://"])
            self.proxies = parent.proxies
            self.verify = parent.verify
         */
    }
}

        