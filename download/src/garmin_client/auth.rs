
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use log::{error, debug, info};
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

#[derive(Default, Deserialize)]
struct TokenInfo {
    token_key: String,
    token_secret: String
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

#[derive(Default, Deserialize)]
#[allow(dead_code)]
pub struct OAuth2Token {
    scope: String,
    jti: String,
    pub access_token: String,
    token_type: String,
    refresh_token: String,
    expires_in: u64,
    refresh_token_expires_in: u64
}
#[derive(Default)]
pub struct OAuth2TokenWrapper {
    pub oauth2_token: OAuth2Token,
    expires_at: u64,
    refresh_token_expires_at: u64
}

#[allow(dead_code)]
impl OAuth2TokenWrapper {
    fn update(&mut self) {
        // update our expirations based on current values
        let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        self.expires_at = now_secs + self.oauth2_token.expires_in;
        self.refresh_token_expires_at = now_secs + self.oauth2_token.refresh_token_expires_in;
    }
    fn expired(&self) -> bool {
        return self.expires_at < SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    fn refresh_expired(&self) -> bool {
        return self.refresh_token_expires_at < SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    fn to_string(&self) -> String {
        format!("{} {}", self.oauth2_token.token_type, self.oauth2_token.access_token)
    }
}

#[allow(dead_code)]
pub struct GaminOAuthManager {
    oauth_consumer_url: String,
    oauth_consumer: HashMap<String, String>,
    user_agent: Vec<String>,
    consumer_info: ConsumerInfo,
    token_info: TokenInfo,
    oauth1_token: OAuth1Token,
    pub oauth2_token: OAuth2TokenWrapper,
    oauth1_client: reqwest::Client
}

impl GaminOAuthManager {
    pub fn new () -> GaminOAuthManager {
        GaminOAuthManager {
            oauth_consumer_url: String::from("https://thegarth.s3.amazonaws.com/oauth_consumer.json"),
            oauth_consumer: HashMap::new(),
            user_agent: vec!["User-Agent".to_owned(), "com.garmin.android.apps.connectmobile".to_owned()],
            consumer_info: Default::default(),
            token_info: Default::default(),
            oauth1_token: Default::default(),
            oauth2_token: Default::default(),
            oauth1_client: reqwest::Client::new()
        }
    }

    pub fn get_oauth1_token(&self) -> &OAuth1Token {
        &self.oauth1_token
    }

    pub fn set_oauth1_token(&mut self, ticket: &str) -> Result<String, reqwest_oauth1::Error> {
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
        endpoint_reqtoken.push_str("&login-url=https://sso.garmin.com/sso/embed&accepts-mfa-tokens=true");

        debug!("====================================================");
        debug!("OAuth1.0 endpoint: {}", &endpoint_reqtoken);
        debug!("====================================================");

        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", "com.garmin.android.apps.connectmobile".parse().unwrap());

        let client = reqwest::Client::new();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let future = rt.block_on({
            let response = client
                .oauth1(secrets)
                .post(&endpoint_reqtoken)
                .headers(headers)
                .send()
                .parse_oauth_token();
            response
        });

        let token: TokenResponse = future.unwrap();
        self.token_info.token_key = String::from(&token.oauth_token);
        self.token_info.token_secret = String::from(&token.oauth_token_secret);

        println!("your token and secret is: \n token: {}\n secret: {}", token.oauth_token, token.oauth_token_secret);

        Ok(token.oauth_token)
    }

    pub fn get_oauth2_token(&self) -> &OAuth2TokenWrapper {
        &self.oauth2_token
    }

    pub fn set_oauth2_token(&mut self) -> Result<bool, reqwest_oauth1::Error> {

        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", "com.garmin.android.apps.connectmobile".parse().unwrap());
        headers.insert("Content-Type", "application/x-www-form-urlencoded".parse().unwrap());

        // TODO: handle MFA at some point
        // TODO: add timeout at some point

        let secrets = reqwest_oauth1::Secrets::new(String::from(&self.consumer_info.consumer_key), String::from(&self.consumer_info.consumer_secret))
            .token(String::from(&self.token_info.token_key), String::from(&self.token_info.token_secret));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::new();
        let future = rt.block_on({
            client
                .oauth1(secrets)
                .post("https://connectapi.garmin.com/oauth-service/oauth/exchange/user/2.0")
                .headers(headers)
                .send()
        });

        match future {
            Ok(resp) => {
                let text_future = rt.block_on(resp.text());
                match text_future {
                    Ok(s) => {
                        debug!("Got oauth2.0 response body: {}", s);
                        self.oauth2_token.oauth2_token = serde_json::from_str(&s).unwrap();
                        self.oauth2_token.update();
                        info!("OAuth2.0 refresh expires in {} secs", self.oauth2_token.oauth2_token.expires_in);
                    }
                    Err(e) => {error!("Expected to get response body. Error: {:?}", e); }
                }
            },
            Err(e) => {error!("Unable to post oauth2.0 request. Error: {:?}", e); }
        }
        Ok(true)
    }
}

        