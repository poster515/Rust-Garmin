
use std::collections::HashMap;
use reqwest::StatusCode;
use serde_json::Value;
use serde_json::json;
use log::debug;
use reqwest::Client;
use reqwest::header::HeaderMap;

use cognito_srp::SrpClient;

// const SESSION_FILE: &str = ".otf_session.json";

/// This struct understands the garmin authentication flow and obtains
/// an OAuth2.0 access token given a username and password. After 
/// authenticating, use the api_request() method to obtain various
/// json and FIT file downloads, and optionally save to file.

// So far I've had to re-use this integer from snooped login sessions and it seems to work. 
// Really need to figure out how to generate this though.
const SRP_A: &str = "REALLY_BIG_INTEGER";

// concatenate the user ID at end of this proxy url to get desired functionality.
const PROXY_URL: &str = "https://api.orangetheory.co/virtual-class/proxy-cors/?url=https://api.orangetheory.co/member/members/";

// need 'Authorization' header key with access token from cognito login
const ALL_WORKOUTS_URL: &str = "https://api.orangetheory.co/virtual-class/in-studio-workouts";

// need 'Authorization' header key with access token from cognito login, and json payload:
// {"ClassHistoryUUId":"class-uuid","MemberUUId":"member-uuid"}
const WORKOUT_SUMMARY_URL: &str = "https://performance.orangetheory.co/v2.4/member/workout/summary";

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
        headers.insert("Referer", "https://otlive.orangetheory.com/".parse().unwrap());

        headers
    }

    async fn get_challenge_params(&mut self, auth_params: HashMap<String, String>) {
        // so far this function works, but SRP_A value is taken from snooped session.
        let auth_url = "https://cognito-idp.us-east-1.amazonaws.com/";

        debug!("Attempting to authenticate via: '{}'", auth_url);

        let body: Value = json!({
            "AuthFlow": "USER_SRP_AUTH",
            "ClientId": "65knvqta6p37efc2l3eh26pl5o",
            "ClientMetadata": {},
            "AuthParameters": {
                "USERNAME": auth_params["USERNAME"],
                "SRP_A": auth_params["SRP_A"]
            }
        });

        debug!("Sending auth request body: {}", serde_json::to_string_pretty(&body).unwrap());

        let mut headers = self.generate_header();
        headers.insert("Access-Control-Request-Headers", "content-type,x-amz-target,x-amz-user-agent".parse().unwrap());
        headers.insert("Access-Control-Request-Method", "POST".parse().unwrap());

        let response = self.client
            .post(auth_url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .unwrap();
        
        let code = response.status();
        self.last_sso_resp_text = response.text().await.unwrap();
        if code != StatusCode::OK {
            let json_response: HashMap<String, Value> = serde_json::from_str(&self.last_sso_resp_text).unwrap();
            debug!("Got code {} and API response: {:?}", code, serde_json::to_string_pretty(&json_response).unwrap());
        }
        
    }

    async fn respond_to_challenge(&mut self, challenge_responses: HashMap<String, String>) {
        let auth_url: &str = "https://cognito-idp.us-east-1.amazonaws.com/";
        let body: Value = json!({
            "ChallengeName": "PASSWORD_VERIFIER",
            "ClientId": "65knvqta6p37efc2l3eh26pl5o",
            "ClientMetadata": {},
            "ChallengeResponses": {
                "USERNAME": challenge_responses["USERNAME"],
                "DEVICE_KEY": "us-east-1_aaa5a071-ff9f-4839-a99e-6e6c7fed51b3",
                "PASSWORD_CLAIM_SECRET_BLOCK": challenge_responses["PASSWORD_CLAIM_SECRET_BLOCK"],
                "PASSWORD_CLAIM_SIGNATURE": challenge_responses["PASSWORD_CLAIM_SIGNATURE"],
                "TIMESTAMP": challenge_responses["TIMESTAMP"]
            }
        });

        debug!("Sending challenge response body: {}", serde_json::to_string_pretty(&body).unwrap());

        let mut headers = self.generate_header();
        headers.insert("X-Amz-Target", "AWSCognitoIdentityProviderService.RespondToAuthChallenge".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let response = self.client
            .post(auth_url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .unwrap();

        let code = response.status();
        self.last_sso_resp_text = response.text().await.unwrap();
        if code != StatusCode::OK {
            let json_response: HashMap<String, Value> = serde_json::from_str(&self.last_sso_resp_text).unwrap();
            debug!("Got code {} and API response: {:?}", code, serde_json::to_string_pretty(&json_response).unwrap());
        }

    }

    /// The first main interface - requires just a username and password,
    /// and obtains an API access token.
    pub async fn login(&mut self, email: &str, password: &str) -> () {
        // if we have a valid token then continue to use it
        // if self.retrieve_json_session() {
        //     return;
        // }
        let srp_client = SrpClient::new(
            email,
            password,
            "aaa5a071-ff9f-4839-a99e-6e6c7fed51b3_us-east-1",
            "65knvqta6p37efc2l3eh26pl5o",
            None,
        );

        // get challenge from server
        let auth_params: HashMap<String, String> = srp_client.get_auth_params().unwrap();
        self.get_challenge_params(auth_params).await;

        // respond to challenge
        let json_response: HashMap<String, Value> = serde_json::from_str(&self.last_sso_resp_text).unwrap();
        let challenge_params: HashMap<String, String> = serde_json::from_value::<HashMap<String, String>>(json_response.get("ChallengeParameters").unwrap().clone())
            .unwrap()
            .clone();
        let challenge_responses = srp_client.process_challenge(challenge_params).unwrap();
        self.respond_to_challenge(challenge_responses).await;

        
        // self.save_json_session();
    }
}