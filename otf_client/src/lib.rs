
use std::collections::HashMap;
use log::debug;
use reqwest::Client;
use reqwest::header::HeaderMap;
// https://api.orangetheory.co/virtual-class/proxy-cors/?url=https://api.orangetheory.co/member/
const AUTH_API: &str = "api.orangetheory.co/virtual-class/proxy-cors/";
const MEMBER_API: &str = "api.orangetheory.co/member";
// const SESSION_FILE: &str = ".otf_session.json";

/// This struct understands the garmin authentication flow and obtains
/// an OAuth2.0 access token given a username and password. After 
/// authenticating, use the api_request() method to obtain various
/// json and FIT file downloads, and optionally save to file.
/// 
/// This client is intended for use with the garmin_download crate, which
/// is already configured with the various garmin backends, although that
/// integration is obviously not required to operate this client separately.
#[allow(dead_code)]
pub struct OtfClient {
    client: Client,
    auth_host: String,
    last_sso_resp_url: String,
    last_sso_resp_text: String,
    last_api_resp_url: String,
    last_api_resp_text: String
}

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

    fn build_auth_url(&self) -> String {

        let mut ub = url_builder::URLBuilder::new();
        ub.set_protocol("https")
            .set_host(MEMBER_API)
            .add_route("cognito")
            .add_route("token");

        let url = ub.build();

        ub = url_builder::URLBuilder::new();
        ub.set_protocol("https")
            .set_host(AUTH_API)
            .add_param("url", &url);
        ub.build()
    }

    /// The first main interface - requires just a username and password,
    /// and obtains an OAuth2.0 access token.
    pub async fn login(&mut self, email: &str, password: &str) -> () {
        // if we have a valid token then continue to use it
        // if self.retrieve_json_session() {
        //     return;
        // }
        let auth_url = self.build_auth_url();
        debug!("Attempting to authenticate via: '{}'", auth_url);

        let body: HashMap<&str, &str> = HashMap::from([
            ("userPoolOutputKey", "MembersCognitoUserPoolId"),
            ("username", email),
            ("password", password),
            ("type", "Cognito")
        ]);
        
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", "com.garmin.android.apps.connectmobile".parse().unwrap());
        headers.insert("Accept", "application/json".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let response = self.client
            .post(&auth_url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .unwrap();

        self.last_sso_resp_text = response.text().await.unwrap();

        debug!("Got API response: {:?}", self.last_sso_resp_text);

        // self.set_oauth1_token(&ticket);
        // if !(self.set_oauth2_token()){
        //     return;
        // }
        // self.save_json_session();
    }
}