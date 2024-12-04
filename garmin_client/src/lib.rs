use log::{debug, error, info, warn};
use regex::Regex;
use reqwest::header::HeaderMap;
use reqwest::{Client, Response};
use serde_json::Value;
use std::cmp::min;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{stdin, stdout};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use zip;

mod auth;

pub const SESSION_FILE: &str = ".garmin_session.json";

/// Basic set of public functions required to use this client.
pub trait ClientTraits {
    fn login(&mut self, username: &str, password: &str) -> bool;
    fn api_request(&mut self, endpoint: &str) -> ();
}

/// This struct understands the garmin authentication flow and obtains
/// an OAuth2.0 access token given a username and password. After
/// authenticating, use the api_request() method to obtain various
/// json and FIT file downloads, and optionally save to file.
///
/// This client is intended for use with the garmin_download crate, which
/// is already configured with the various garmin backends, although that
/// integration is obviously not required to operate this client separately.
// #[allow(dead_code)]
pub struct GarminClient {
    client: Client,
    auth_host: String,
    api_host: String,
    last_sso_resp_url: String,
    last_sso_resp_text: String,
    last_api_resp_url: String,
    last_api_resp_text: String,
    oauth_manager: auth::GaminOAuthManager,
}

impl GarminClient {
    // shamelessly adopted from:
    // https://github.com/matin/garth/blob/main/garth/sso.py
    pub fn new() -> GarminClient {
        GarminClient {
            client: Client::builder().cookie_store(true).build().unwrap(),
            auth_host: String::from("sso.garmin.com"),
            api_host: String::from("connectapi.garmin.com"),
            last_sso_resp_url: String::new(),
            last_sso_resp_text: String::new(),
            last_api_resp_url: String::new(),
            last_api_resp_text: String::new(),
            oauth_manager: auth::GaminOAuthManager::new(),
        }
    }

    fn build_auth_url(&self, endpoint: &str) -> String {
        // build the main rqeuest URL with provided routes
        let mut ub = url_builder::URLBuilder::new();
        ub.set_protocol("https")
            .set_host(&self.auth_host)
            .add_route("sso")
            .add_route("embed");
        let sso_embed = ub.build();

        let mut ub = url_builder::URLBuilder::new();
        ub.set_protocol("https")
            .set_host(&self.auth_host)
            .add_route("sso")
            .add_route(endpoint)
            .add_param("id", "gauth-widget")
            .add_param("embedWidget", "true")
            .add_param("gauthHost", &sso_embed[..])
            .add_param("service", &sso_embed[..])
            .add_param("source", &sso_embed[..])
            .add_param("redirectAfterAccountLoginUrl", &sso_embed[..])
            .add_param("redirectAfterAccountCreationUrl", &sso_embed[..]);
        ub.build()
    }

    fn build_api_url(&self, endpoint: &str) -> String {
        let mut ub = url_builder::URLBuilder::new();
        ub.set_protocol("https")
            .set_host(&self.api_host)
            .add_route(endpoint);
        ub.build()
    }

    async fn set_cookie(&mut self) -> bool {
        /*
        Called before actual login so we can get csrf token.
        */
        let mut ub = url_builder::URLBuilder::new();
        ub.set_protocol("https")
            .set_host(&self.auth_host)
            .add_route("sso");
        let gauth_host = ub.build();

        let mut ub = url_builder::URLBuilder::new();
        ub.set_protocol("https")
            .set_host(&self.auth_host)
            .add_route("sso")
            .add_route("embed")
            .add_param("id", "gauth-widget")
            .add_param("embedWidget", "true")
            .add_param("gauthHost", &gauth_host);
        let url = ub.build();

        debug!("====================================================");
        debug!("Requesting url for cookies: {}", url);
        debug!("====================================================");

        let response = self.client.get(&url).send().await.unwrap();
        self.last_sso_resp_url = response.url().to_string();
        self.last_sso_resp_text = response.text().await.unwrap();
        true
    }

    async fn get_csrf_token(&mut self) -> String {
        let url: String = self.build_auth_url("signin");

        debug!("====================================================");
        debug!("Requesting url for csrf: {}", url);
        debug!("====================================================");

        let mut headers = HeaderMap::new();
        headers.insert("referer", self.last_sso_resp_url.as_str().parse().unwrap());
        headers.insert("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 16_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148".parse().unwrap());

        let response = self.client.get(&url).headers(headers).send().await.unwrap();
        self.last_sso_resp_url = response.url().to_string();
        self.last_sso_resp_text = response.text().await.unwrap();
        self.parse_csrf_token(&self.last_sso_resp_text)
    }

    async fn submit_login(&mut self, username: &str, password: &str, csrf_token: &str) -> bool {
        // is this broken? I'm to get csrf token and get here, but am getting a 500 internal server
        // error when making this request.
        let url = self.build_auth_url("signin");

        debug!("====================================================");
        debug!("Requesting url for login: {}", url);
        debug!("====================================================");

        let mut headers = HeaderMap::new();
        headers.insert("referer", self.last_sso_resp_url.as_str().parse().unwrap());
        headers.insert("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 16_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148".parse().unwrap());

        // these need to be sent as request body
        let form = HashMap::from([
            ("username", String::from(username)),
            ("password", String::from(password)),
            ("embed", String::from("true")),
            ("_csrf", String::from(csrf_token)),
        ]);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .form(&form)
            .send()
            .await
            .unwrap();

        self.last_sso_resp_url = response.url().to_string();
        self.last_sso_resp_text = response.text().await.unwrap();

        debug!("====================================================");
        debug!("Got login reponse: {}", self.last_sso_resp_text);
        debug!("====================================================");
        true
    }

    fn parse_csrf_token(&self, response_html: &String) -> String {
        let re = Regex::new(r#"name="_csrf"\s+value="(\w+)"#).unwrap();
        for (_, [csrf]) in re.captures_iter(&response_html).map(|c| c.extract()) {
            debug!("====================================================");
            debug!("Found csrf token: {}", csrf);
            debug!("====================================================");
            return String::from(csrf);
        }
        error!("====================================================");
        panic!("Unable to find csrf token in body: {}", response_html);
    }

    fn parse_title(&self, response_html: &String) -> String {
        let re = Regex::new(r#"<title>(.+?)</title>"#).unwrap();
        for (_, [title]) in re.captures_iter(&response_html).map(|c| c.extract()) {
            debug!("====================================================");
            if title == "Success" {
                info!("Got successful login!");
                return String::from(title);
            } else if title == "GARMIN Authentication Application" {
                // testing shows that this title is received with incorrect credentials.
                error!("Got unsuccessful login :( check your credentials?");
            } else {
                // could possibly have MFA requirement, just return it
                return String::from(title);
            }
            debug!("====================================================");
        }
        error!("====================================================");
        panic!("Unable to find title in body: {}", response_html);
    }

    fn parse_ticket(&self, response_html: &String) -> String {
        let re = Regex::new(r#"embed\?ticket=([^"]+)""#).unwrap();
        for (_, [ticket]) in re.captures_iter(&response_html).map(|c| c.extract()) {
            debug!("====================================================");
            debug!("Found ticket: {}", ticket);
            debug!("====================================================");
            return String::from(ticket);
        }
        error!("====================================================");
        panic!("Unable to find ticket in body: {}", response_html);
    }

    /// The first main interface - requires just a username and password,
    /// and obtains an OAuth2.0 access token. Returns false if unsuccessful.
    pub async fn login(&mut self, username: &str, password: &str) -> bool {
        // if we have a valid token then continue to use it
        if self.retrieve_json_session() {
            return true;
        }

        // set cookies (looks like this still works)
        if !self.set_cookie().await {
            error!("Unable to set session, cannot proceed with authentication");
            return false;
        }

        // get csrf token (appears to work still as well, although not 100% its correct)
        let csrf_token: String = self.get_csrf_token().await;

        if csrf_token.len() == 0 {
            panic!(
                "Unable to find CSRF token in response: {}",
                &self.last_sso_resp_text
            );
        }

        // Submit login form with email and password
        self.submit_login(username, password, &csrf_token).await;
        let mut title = self.parse_title(&self.last_sso_resp_text);
        if title.len() == 0 {
            panic!(
                "Unable to find 'title' field in response: {}",
                &self.last_sso_resp_text
            );
        }

        // handle any MFA for user
        if title.contains("MFA") {
            self.handle_mfa().await;
            title = self.parse_title(&self.last_sso_resp_text);
        }

        if title != "Success" {
            error!("Unable to authenticate user!");
            return false;
        }

        let ticket = self.parse_ticket(&self.last_sso_resp_text);
        if ticket.len() == 0 {
            return false;
        }

        self.set_oauth1_token(&ticket).await;
        if !(self.set_oauth2_token().await) {
            return false;
        }
        self.save_json_session();
        true
    }

    async fn handle_mfa(&mut self) {
        let csrf_token: String = self.get_csrf_token().await;

        let mut mfa_code = String::new();
        print!("Enter MFA code: ");
        let _ = stdout().flush();
        stdin()
            .read_line(&mut mfa_code)
            .expect("Did not enter a correct string");

        let mut headers = HeaderMap::new();
        headers.insert("referer", self.last_sso_resp_url.as_str().parse().unwrap());
        headers.insert("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 16_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148".parse().unwrap());

        let form = HashMap::from([
            ("mfa-code", String::from(mfa_code)),
            ("fromPage", String::from("setupEnterMfaCode")),
            ("embed", String::from("true")),
            ("_csrf", String::from(csrf_token)),
        ]);

        let url = self.build_auth_url("verifyMFA/loginEnterMfaCode");

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .form(&form)
            .send()
            .await
            .unwrap();

        self.last_sso_resp_url = response.url().to_string();
        self.last_sso_resp_text = response.text().await.unwrap();
    }

    async fn set_oauth1_token(&mut self, ticket: &str) {
        let oauth1_token: String = self
            .oauth_manager
            .set_oauth1_token(ticket, self.client.clone())
            .await
            .unwrap();
        info!("Got oauth1 token: {}", oauth1_token);
    }

    async fn set_oauth2_token(&mut self) -> bool {
        match self
            .oauth_manager
            .set_oauth2_token(self.client.clone())
            .await
        {
            Ok(token) => {
                info!("Got oauth2 token: {}", token);
                true
            }
            Err(e) => {
                error!("Unable to obtain oauth2_token: {}", e);
                false
            }
        }
    }

    /// After logging in, use this API interface to download data. Some URLs download
    /// json data, and some download zip files (that are auto-extracted into FIT files here).
    ///
    /// Users need to specify the expected data response type via 'json_or_binary' to
    /// determine how to process the raw data.
    ///
    /// By specifying filepath=None, the data is not saved to file. JSON text responses
    /// can be retrieved via the get_last_resp_text() method; however binary (i.e., FIT file)
    /// downloads are dropped if not saved to file currently.
    pub async fn api_request(
        &mut self,
        endpoint: &str,
        params: Option<HashMap<&str, &str>>,
        json_or_binary: bool,
        filepath: Option<String>,
    ) -> bool {
        // use for actual application data downloads
        let url = self.build_api_url(endpoint);

        if self.oauth_manager.get_oauth2_token().is_expired() {
            info!("====================================================");
            info!("ConnectAPI refreshing OAuth2.0 token...");
            info!("====================================================");
            self.set_oauth2_token().await;
        }

        let access_token: String = String::from(
            &self
                .oauth_manager
                .get_oauth2_token()
                .oauth2_token
                .access_token,
        );

        debug!("ConnectAPI requesting from: {}", &url);

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", access_token).parse().unwrap(),
        );

        let mut builder = self.client.get(url).headers(headers);

        match params {
            Some(param_map) => {
                builder = builder.query(&param_map);
            }
            None => {}
        }

        let response = builder.send().await.unwrap();

        if json_or_binary {
            self.last_api_resp_url = response.url().to_string();
            self.last_api_resp_text = response.text().await.unwrap();
            match filepath {
                Some(filename) => {
                    self.save_as_json(&self.last_api_resp_text, filename);
                    true
                }
                None => {
                    debug!(
                        "Got api response: {}",
                        &self.last_api_resp_text[0..min(1024, self.last_api_resp_text.len())]
                    );
                    true
                }
            }
        } else {
            match filepath {
                Some(filename) => {
                    self.save_as_binary(response, filename).await;
                    true
                }
                None => {
                    debug!(
                        "Got {} bytes of binary response, ignoring",
                        response.content_length().unwrap_or(0)
                    );
                    false
                }
            }
        }
    }

    fn save_as_json(&self, data: &str, filepath: String) {
        if data.len() == 0 {
            return;
        }
        match File::create(&filepath) {
            Ok(file) => {
                let mut writer = BufWriter::new(file);
                let json_data: HashMap<String, serde_json::Value> =
                    serde_json::from_str(data).unwrap();
                match serde_json::to_writer_pretty(&mut writer, &json_data) {
                    Ok(_) => match writer.flush() {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error flushing writer: {}", e);
                        }
                    },
                    Err(e) => {
                        error!("Error writing json to buffer: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Unable to create file {}, error: {}", filepath, e);
            }
        }
    }

    async fn save_as_binary(&self, mut response: Response, filepath: String) {
        // .FIT files are saved as .ZIP files FYI
        let mut num_chunks = 0;
        match File::create(&filepath) {
            Ok(mut file) => {
                while let Ok(Some(chunk)) = response.chunk().await {
                    match file.write(&chunk) {
                        Ok(_) => {
                            num_chunks += 1;
                            info!(
                                "Wrote chunk #{} to {}. Size: {}",
                                num_chunks,
                                &filepath,
                                &chunk.len()
                            );
                        }
                        Err(e) => {
                            error!(
                                "Error writing chunk #{} to {}, error: {}",
                                num_chunks, &filepath, e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                error!("Unable to create file {}, error: {}", &filepath, e);
            }
        }
        if num_chunks == 0 {
            warn!("Didn't save any binary zip file data");
            return;
        }
        // now unzip the downloaded zip
        info!("Attempting to unzip files...");
        let file = File::open(&filepath).unwrap();
        match zip::ZipArchive::new(file) {
            Ok(mut archive) => {
                for i in 0..archive.len() {
                    let mut file = archive.by_index(i).unwrap();
                    let mut buffer: Vec<u8> = vec![];
                    std::io::copy(&mut file, &mut buffer)
                        .expect("Unable to copy archive file contents to buffer :(");

                    // get folder from filepath
                    let new_path = Path::new(&filepath).parent().unwrap().join(&file.name());
                    info!("Saving FIT file contents: {}", new_path.display());
                    fs::write(new_path, &buffer).expect("Unable to write FIT file contents :(");
                }
            }
            Err(e) => {
                error!("Unable to unzip file {}, error: {}", &filepath, e);
            }
        }
    }

    /// Sets the token and expiration value from a HashMap.
    ///
    /// Returns true if valid access_token found
    fn retrieve_json_session(&mut self) -> bool {
        match fs::read_to_string(&SESSION_FILE) {
            Ok(file_contents) => {
                let map: Value = serde_json::from_str(&file_contents).unwrap();
                let expires_at_str = map["expires_at"].to_string().replace('"', "");
                let expiration: u64 = expires_at_str.parse::<u64>().unwrap();
                if expiration
                    < SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                {
                    warn!("Found garmin session token, but it expired. Need to re-authenticate");
                    return false;
                }
                info!("Successfully read garmin session token!");
                self.oauth_manager.oauth2_token.expires_at = expiration;
                self.oauth_manager.oauth2_token.oauth2_token.access_token =
                    map["token"].to_string();
                return true;
            }
            Err(e) => {
                info!("Unable opening garmin_client session file: {}", e);
            }
        }
        false
    }
    /// Saves the current access token if valid
    fn save_json_session(&self) {
        match File::create(&SESSION_FILE) {
            Ok(file) => {
                let mut writer = BufWriter::new(file);
                let json_data: HashMap<String, String> = HashMap::from([
                    (
                        String::from("expires_at"),
                        format!("{}", self.oauth_manager.oauth2_token.expires_at),
                    ),
                    (
                        String::from("token"),
                        String::from(&self.oauth_manager.oauth2_token.oauth2_token.access_token),
                    ),
                ]);
                match serde_json::to_writer_pretty(&mut writer, &json_data) {
                    Ok(_) => match writer.flush() {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error flushing writer: {}", e);
                        }
                    },
                    Err(e) => {
                        error!("Error writing json to buffer: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Unable to create file {}, error: {}", SESSION_FILE, e);
            }
        }
    }

    /// When specifying a JSON download in the api_request() function, this
    /// can be called to return that JSON text (use in lieu of saving JSON to file).
    pub fn get_last_resp_text(&self) -> &str {
        &self.last_api_resp_text
    }
}
