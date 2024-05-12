
use std::collections::HashMap;
use reqwest::StatusCode;
use serde_json::Value;
use serde_json::json;
use log::debug;
use reqwest::Client;
use reqwest::header::HeaderMap;


// const SESSION_FILE: &str = ".otf_session.json";

/// This struct understands the garmin authentication flow and obtains
/// an OAuth2.0 access token given a username and password. After 
/// authenticating, use the api_request() method to obtain various
/// json and FIT file downloads, and optionally save to file.

// So far I've had to re-use this integer from snooped login sessions and it seems to work. 
// Really need to figure out how to generate this though.
const SRP_A: &str = "REALLY_BIG_INTEGER";

#[allow(dead_code)]
pub struct OtfClient {
    client: Client,
    auth_host: String,
    last_sso_resp_url: String,
    last_sso_resp_text: String,
    last_api_resp_url: String,
    last_api_resp_text: String
}

#[allow(dead_code, unused_variables)]
impl OtfClient {
    pub fn new() -> OtfClient {
        OtfClient {
            client: Client::builder().cookie_store(true).build().unwrap(),
            auth_host: String::from("https://cognito-idp.us-east-1.amazonaws.com"),
            last_sso_resp_url: String::new(),
            last_sso_resp_text: String::new(),
            last_api_resp_url: String::new(),
            last_api_resp_text: String::new()
        }
    }

    fn generate_header(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", "com.garmin.android.apps.connectmobile".parse().unwrap());
        headers.insert("Accept", "*/*".parse().unwrap());
        headers.insert("Content-Type", "application/x-amz-json-1.1".parse().unwrap());
        headers.insert("X-Amz-Target", "AWSCognitoIdentityProviderService.InitiateAuth".parse().unwrap());
        headers.insert("X-Amz-User-Agent", "aws-amplify/0.1.x js".parse().unwrap());
        headers.insert("Origin", "https://otlive.orangetheory.com".parse().unwrap());
        headers.insert("Referer", "https://otlive.orangetheory.com".parse().unwrap());
        headers.insert("Access-Control-Request-Headers", "content-type,x-amz-target,x-amz-user-agent".parse().unwrap());
        headers.insert("Access-Control-Request-Method", "POST".parse().unwrap());

        headers
    }

    async fn get_challenge_params(&mut self, email: &str) {
        // so far this function works, but SRP_A value is taken from snooped session.
        let auth_url = "https://cognito-idp.us-east-1.amazonaws.com/";

        debug!("Attempting to authenticate via: '{}'", auth_url);

        let body: Value = json!({
            "AuthFlow": "USER_SRP_AUTH",
            "ClientId": "65knvqta6p37efc2l3eh26pl5o",
            "ClientMetadata": {},
            "AuthParameters": {
                "USERNAME": email,
                "SRP_A": SRP_A
            }
        });

        let headers = self.generate_header();
        let response = self.client
            .post(auth_url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .unwrap();

        if response.status() != StatusCode::OK {
            debug!("Got code {} and API response: {:?}", response.status(), self.last_sso_resp_text);
        }

        self.last_sso_resp_text = response.text().await.unwrap();
        
    }

    fn generate_password_claim_signature(&self, auth_params: &HashMap<&str, String>) -> String {
        
        let device_password: &str = &auth_params["DEVICE_PASSWORD"];
        let srp_b: &str = &auth_params["SRP_B"];
        let salt: &str = &auth_params["SALT"];
        let timestamp: &str = &auth_params["TIMESTAMP"];
        let secret_block: &str = &auth_params["SECRET_BLOCK"];

        // TODO: finish

        String::new()
    }

    async fn respond_to_challenge(&mut self, auth_params: &HashMap<&str, String>) {
        let auth_url: &str = "https://cognito-idp.us-east-1.amazonaws.com/";
        let signature: String = self.generate_password_claim_signature(auth_params);
        let body: Value = json!({
            "ChallengeName": "PASSWORD_VERIFIER",
            "ClientId": "65knvqta6p37efc2l3eh26pl5o",
            "ClientMetadata": {},
            "ChallengeResponses": {
                "USERNAME": "107494a1-d531-4f9e-8f78-da047b4414ce",
                "DEVICE_KEY": "us-east-1_aaa5a071-ff9f-4839-a99e-6e6c7fed51b3",
                "PASSWORD_CLAIM_SECRET_BLOCK": auth_params["SALT"],
                "PASSWORD_CLAIM_SIGNATURE": signature,
                "TIMESTAMP": "Fri May 10 23:32:24 UTC 2024"
            }
        });

        let mut headers = self.generate_header();
        headers.insert("X-Amz-Target", "AWSCognitoIdentityProviderService.RespondToAuthChallenge".parse().unwrap());

        let response = self.client
            .post(auth_url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .unwrap();
        if response.status() != StatusCode::OK {
            debug!("Got code {} and API response: {:?}", response.status(), self.last_sso_resp_text);
        }

        self.last_sso_resp_text = response.text().await.unwrap();
    }

    /// The first main interface - requires just a username and password,
    /// and obtains an OAuth2.0 access token.
    pub async fn login(&mut self, email: &str, password: &str) -> () {
        // if we have a valid token then continue to use it
        // if self.retrieve_json_session() {
        //     return;
        // }
        self.get_challenge_params(email).await;

        let json_response: Value = serde_json::from_str(&self.last_sso_resp_text).unwrap();
        let mut auth_params: HashMap<&str, String> = HashMap::new();
        auth_params.insert("SALT", json_response["ChallengeParameters"]["SALT"].to_string());
        auth_params.insert("SECRET_BLOCK", json_response["ChallengeParameters"]["SECRET_BLOCK"].to_string());
        auth_params.insert("SRP_B", json_response["ChallengeParameters"]["SRP_B"].to_string());
        auth_params.insert("USERNAME", json_response["ChallengeParameters"]["USERNAME"].to_string());

        self.respond_to_challenge(&auth_params).await;
        
        // self.save_json_session();
    }
}