use chrono::{DateTime, Local};
use log::{debug, info};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest;
use reqwest::header::HeaderMap;

use reqwest_oauth1;
use reqwest_oauth1::{OAuthClientProvider, TokenReaderError, TokenReaderResult, TokenResponse};

use serde::Deserialize;

const OAUTH_TOKEN_KEY: &str = "oauth_token";
const OAUTH_TOKEN_SECRET_KEY: &str = "oauth_token_secret";

#[derive(Default, Deserialize)]
struct ConsumerInfo {
    consumer_key: String,
    consumer_secret: String,
}

#[derive(Default, Deserialize)]
struct TokenInfo {
    token_key: String,
    token_secret: String,
}

#[derive(Default)]
#[allow(dead_code)]
pub struct OAuth1Token {
    token_info: TokenInfo,
    mfa_token: String,
    mfa_expiration_timestamp: DateTime<Local>,
    domain: String,
}

#[derive(Default, Deserialize)]
#[allow(dead_code)] // need to deserialize message body into this struct
pub struct OAuth2Token {
    scope: String,
    jti: String,
    pub access_token: String,
    token_type: String,
    refresh_token: String,
    expires_in: u64,
    refresh_token_expires_in: u64,
}
#[derive(Default)]
pub struct OAuth2TokenWrapper {
    pub oauth2_token: OAuth2Token,
    pub expires_at: u64,
    refresh_token_expires_at: u64,
}

// #[allow(dead_code)]
impl OAuth2TokenWrapper {
    fn update(&mut self) {
        // update our expirations based on current values
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.expires_at = now_secs + self.oauth2_token.expires_in;
        self.refresh_token_expires_at = now_secs + self.oauth2_token.refresh_token_expires_in;
    }
    pub fn is_expired(&self) -> bool {
        self.expires_at
            < SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
    }
}

// copied from the reqwest oauth1 crate because it's hidden behind private::sealed trait.
fn read_oauth_token(text: String) -> TokenReaderResult<TokenResponse> {
    let mut destructured = text
        .split("&")
        .map(|e| e.splitn(2, "="))
        .map(|v| {
            let mut iter = v.into_iter();
            (
                iter.next().unwrap_or_default().to_string(),
                iter.next().unwrap_or_default().to_string(),
            )
        })
        .collect::<HashMap<String, String>>();
    let oauth_token = destructured.remove(OAUTH_TOKEN_KEY);
    let oauth_token_secret = destructured.remove(OAUTH_TOKEN_SECRET_KEY);
    match (oauth_token, oauth_token_secret) {
        (Some(t), Some(s)) => Ok(TokenResponse {
            oauth_token: t,
            oauth_token_secret: s,
            remain: destructured,
        }),
        (None, _) => Err(TokenReaderError::TokenKeyNotFound(OAUTH_TOKEN_KEY, text)),
        (_, _) => Err(TokenReaderError::TokenKeyNotFound(
            OAUTH_TOKEN_SECRET_KEY,
            text,
        )),
    }
}

pub struct GaminOAuthManager {
    oauth_consumer_url: String,
    consumer_info: ConsumerInfo,
    oauth1_token: OAuth1Token,
    pub oauth2_token: OAuth2TokenWrapper,
}

impl GaminOAuthManager {
    pub fn new() -> GaminOAuthManager {
        GaminOAuthManager {
            oauth_consumer_url: String::from(
                "https://thegarth.s3.amazonaws.com/oauth_consumer.json",
            ),
            consumer_info: Default::default(),
            oauth1_token: Default::default(),
            oauth2_token: Default::default(),
        }
    }

    pub async fn set_oauth1_token(
        &mut self,
        ticket: &str,
        client: reqwest::Client,
    ) -> Result<String, reqwest_oauth1::Error> {
        self.consumer_info = reqwest::get(&self.oauth_consumer_url)
            .await
            .unwrap()
            .json::<ConsumerInfo>()
            .await
            .unwrap();

        let secrets = reqwest_oauth1::Secrets::new(
            &self.consumer_info.consumer_key,
            &self.consumer_info.consumer_secret,
        );

        let mut endpoint_reqtoken: String =
            String::from("https://connectapi.garmin.com/oauth-service/oauth/preauthorized");
        endpoint_reqtoken.push_str("?ticket=");
        endpoint_reqtoken.push_str(ticket);
        endpoint_reqtoken
            .push_str("&login-url=https://sso.garmin.com/sso/embed&accepts-mfa-tokens=true");

        debug!("====================================================");
        debug!("OAuth1.0 endpoint: {}", &endpoint_reqtoken);
        debug!("====================================================");

        let mut headers = HeaderMap::new();
        headers.insert(
            "User-Agent",
            "com.garmin.android.apps.connectmobile".parse().unwrap(),
        );

        let response = client
            .oauth1(secrets)
            .post(&endpoint_reqtoken)
            .headers(headers)
            .send()
            .await
            .unwrap();

        let body_text = response.text().await.unwrap();

        debug!("====================================================");
        debug!("OAuth1.0 response body: {}", &body_text);
        debug!("====================================================");

        let token: TokenResponse = read_oauth_token(body_text).unwrap();
        self.oauth1_token.token_info.token_key = String::from(&token.oauth_token);
        self.oauth1_token.token_info.token_secret = String::from(&token.oauth_token_secret);

        info!("====================================================");
        info!(
            "OAuth1.0 token and secret is: \n token: {}\n secret: {}",
            token.oauth_token, token.oauth_token_secret
        );
        info!("====================================================");

        Ok(token.oauth_token)
    }

    pub fn get_oauth2_token(&self) -> &OAuth2TokenWrapper {
        &self.oauth2_token
    }

    pub async fn set_oauth2_token(
        &mut self,
        client: reqwest::Client,
    ) -> Result<String, reqwest_oauth1::Error> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "User-Agent",
            "com.garmin.android.apps.connectmobile".parse().unwrap(),
        );
        headers.insert(
            "Content-Type",
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        let secrets = reqwest_oauth1::Secrets::new(
            String::from(&self.consumer_info.consumer_key),
            String::from(&self.consumer_info.consumer_secret),
        )
        .token(
            String::from(&self.oauth1_token.token_info.token_key),
            String::from(&self.oauth1_token.token_info.token_secret),
        );

        let response = client
            .oauth1(secrets)
            .post("https://connectapi.garmin.com/oauth-service/oauth/exchange/user/2.0")
            .headers(headers)
            .send()
            .await
            .unwrap();

        let body_text = response.text().await.unwrap();

        self.oauth2_token.oauth2_token = serde_json::from_str(&body_text).unwrap();
        self.oauth2_token.update();
        info!(
            "OAuth2.0 refresh expires in {} secs",
            self.oauth2_token.oauth2_token.expires_in
        );

        Ok(String::from(&self.oauth2_token.oauth2_token.access_token))
    }
}
