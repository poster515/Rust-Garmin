
use std::collections::HashMap;
use std::cmp::min;
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::io::{stdin, stdout};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use log::{error, debug, warn, info};
use regex::Regex;
use reqwest::{Client, Response};
use reqwest::header::HeaderMap;
use serde_json::{Value};
use zip;

mod auth;

pub const SESSION_FILE: &str = ".garmin_session.json";

/// Basic set of public functions required to use this client.
pub trait ClientTraits {
    fn login(&mut self, username: &str, password: &str) -> ();
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
    last_sso_resp_url: String,
    last_sso_resp_text: String,
    last_api_resp_url: String,
    last_api_resp_text: String,
    oauth_manager: auth::GaminOAuthManager,
    runtime: tokio::runtime::Runtime
}

impl GarminClient {
    // shamelessly adopted from:
    // https://github.com/cpfair/tapiriik/blob/master/tapiriik/services/GarminConnect/garminconnect.py#L10
    pub fn new() -> GarminClient {
        GarminClient {
            client: Client::builder().cookie_store(true).build().unwrap(),
            auth_host: String::from("https://sso.garmin.com/sso"),
            last_sso_resp_url: String::new(),
            last_sso_resp_text: String::new(),
            last_api_resp_url: String::new(),
            last_api_resp_text: String::new(),
            oauth_manager: auth::GaminOAuthManager::new(),
            runtime: tokio::runtime::Runtime::new().unwrap()
        }
    }

    fn build_auth_url(&self, routes: Vec<&str>) -> String {
        let mut sso_embed = String::from(&self.auth_host);
        sso_embed.push_str("/embed");

        let mut ub = url_builder::URLBuilder::new();
        ub.set_protocol("https").set_host("sso.garmin.com");

        for route in routes {
            ub.add_route(route);
        }

        ub.add_param("id", "gauth-widget")
            .add_param("embedWidget", "true")
            .add_param("gauthHost", &sso_embed[..])
            .add_param("service", &sso_embed[..])
            .add_param("source", &sso_embed[..])
            .add_param("redirectAfterAccountLoginUrl", &sso_embed[..])
            .add_param("redirectAfterAccountCreationUrl", &sso_embed[..]);
        ub.build()
    }

    fn build_api_url(&self, endpoint: &str) -> url_builder::URLBuilder {

        let mut ub = url_builder::URLBuilder::new();
        ub.set_protocol("https")
            .set_host("connectapi.garmin.com")
            .add_route(endpoint);
        ub
    }

    fn set_cookie(&mut self) -> bool {
        /*
        Called before actual login so we can get csrf token.
        */
        let mut ub = url_builder::URLBuilder::new();
        ub.set_protocol("https")
            .set_host("sso.garmin.com")
            .add_route("sso")
            .add_route("embed")
            .add_param("id", "gauth-widget")
            .add_param("embedWidget", "true")
            .add_param("gauthHost", &self.auth_host);
        let url = ub.build();

        debug!("====================================================");
        debug!("Requesting url: {}", url);
        debug!("====================================================");

        let future = self.runtime.block_on(self.client.get(&url).send());

        let response = future.unwrap();
        self.last_sso_resp_url = response.url().to_string();

        let get_body_future = self.runtime.block_on(response.text());
        self.last_sso_resp_text = get_body_future.unwrap();
        true
    }

    fn get_csrf_token(&mut self) -> String {

        let url = self.build_auth_url(vec!["sso", "signin"]);
        let mut headers = HeaderMap::new();
        headers.insert("referer", self.last_sso_resp_url.as_str().parse().unwrap());

        let future = self.runtime.block_on(self.client.get(&url).headers(headers).send());

        let response = future.unwrap();
        self.last_sso_resp_url = response.url().to_string();

        let get_body_future = self.runtime.block_on(response.text());

        self.last_sso_resp_text = get_body_future.unwrap();
        self.parse_csrf_token(&self.last_sso_resp_text)
    }

    fn submit_login(&mut self, username: &str, password: &str, csrf_token: &str) -> bool {
        let url = self.build_auth_url(vec!["sso", "signin"]);
        let mut headers = HeaderMap::new(); 
        headers.insert("referer", self.last_sso_resp_url.as_str().parse().unwrap());

        let form = HashMap::from([
            ("username", String::from(username)),
            ("password", String::from(password)),
            ("embed", String::from("true")),
            ("_csrf", String::from(csrf_token))
        ]);

        let future = self.runtime.block_on({
            self.client.post(&url)
                .headers(headers)
                .form(&form)
                .send()
        });

        let response = future.unwrap();
        self.last_sso_resp_url = response.url().to_string();

        let get_body_future = self.runtime.block_on(response.text());
        self.last_sso_resp_text = get_body_future.unwrap();
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
        error!("Unable to find csrf token in body: {}", response_html);
        error!("====================================================");
        String::new()
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
        error!("Unable to find title in body: {}", response_html);
        error!("====================================================");
        String::new()
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
        error!("Unable to find ticket in body: {}", response_html);
        error!("====================================================");
        String::new()
    }

    /// The first main interface - requires just a username and password,
    /// and obtains an OAuth2.0 access token.
    pub fn login(&mut self, username: &str, password: &str) -> () {
        // if we have a valid token then continue to use it
        if self.retrieve_json_session() {
            return;
        }

        // set cookies
        if !self.set_cookie() {
            error!("Unable to set session, cannot proceed with authentication");
            return
        }

        // get csrf token
        let csrf_token: String = self.get_csrf_token();
        
        if csrf_token.len() == 0 {
            error!("Unable to find CSRF token in response: {}", &self.last_sso_resp_text);
            return
        }

        // Submit login form with email and password
        self.submit_login(username, password, &csrf_token);
        let mut title = self.parse_title(&self.last_sso_resp_text);
        if title.len() == 0 {
            error!("Unable to find 'title' field in response: {}", &self.last_sso_resp_text);
            return
        }

        // handle any MFA for user
        if title.contains("MFA") {
            self.handle_mfa();
            title = self.parse_title(&self.last_sso_resp_text);
        }

        if title != "Success" {
            error!("Unable to authenticate user!");
            return;
        }

        let ticket = self.parse_ticket(&self.last_sso_resp_text);
        if ticket.len() == 0 {
            return;
        }

        self.set_oauth1_token(&ticket);
        if !(self.set_oauth2_token()){
            return;
        }
        self.save_json_session();
    }

    fn handle_mfa(&mut self) {
        let csrf_token: String = self.get_csrf_token();

        let mut mfa_code = String::new();
        print!("Enter MFA code: ");
        let _ = stdout().flush();
        stdin().read_line(&mut mfa_code).expect("Did not enter a correct string");

        let mut headers = HeaderMap::new();
        headers.insert("referer", self.last_sso_resp_url.as_str().parse().unwrap());

        let form = HashMap::from([
            ("mfa-code", String::from(mfa_code)),
            ("fromPage", String::from("setupEnterMfaCode")),
            ("embed", String::from("true")),
            ("_csrf", String::from(csrf_token))
        ]);

        let url = self.build_auth_url(vec!["sso", "verifyMFA", "loginEnterMfaCode"]);
        let future = self.runtime.block_on({
            self.client.post(&url)
                .headers(headers)
                .form(&form)
                .send()
        });

        let response = future.unwrap();
        self.last_sso_resp_url = response.url().to_string();

        let get_body_future = self.runtime.block_on(response.text());
        self.last_sso_resp_text = get_body_future.unwrap();
    }

    fn set_oauth1_token(&mut self, ticket: &str) {
        let oauth1_token: String = self.oauth_manager.set_oauth1_token(ticket, self.client.clone()).unwrap();
        info!("Got oauth1 token: {}", oauth1_token);
    }

    fn set_oauth2_token(&mut self) -> bool {
        match self.oauth_manager.set_oauth2_token(self.client.clone()) {
            Ok(token) => {
                info!("Got oauth2 token: {}", token);
                true
            }, Err(e) => {
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
    pub fn api_request(&mut self, 
            endpoint: &str, 
            params: Option<HashMap<&str, &str>>,
            json_or_binary: bool,
            filepath: Option<String>) -> bool {
        // use for actual application data downloads
        let url = self.build_api_url(endpoint).build();

        if self.oauth_manager.get_oauth2_token().is_expired() {
            info!("====================================================");
            info!("ConnectAPI refreshing OAuth2.0 token...");
            info!("====================================================");
            self.set_oauth2_token();
        }

        let access_token: String = String::from(&self.oauth_manager.get_oauth2_token().oauth2_token.access_token);

        debug!("ConnectAPI requesting from: {}", &url);

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {}", access_token).parse().unwrap());

        let future = self.runtime.block_on({
            let builder = self.client.get(url).headers(headers);

            match params {
                Some(param_map) => {
                    builder.query(&param_map).send()
                },
                None => {
                    builder.send()
                }
            }
        });

        match future {
            Ok(response) => {
                if json_or_binary {
                    self.last_api_resp_url = response.url().to_string();
                    let get_body_future = self.runtime.block_on({
                        response.text()
                    });

                    match get_body_future {
                        Ok(body) => {
                            self.last_api_resp_text = body;
                            match filepath {
                                Some(filename) => { self.save_as_json(&self.last_api_resp_text, filename); },
                                None => { debug!("Got api response: {}", &self.last_api_resp_text[0..min(1024, self.last_api_resp_text.len())]); }
                            }
                            true
                        }, Err(e) => {
                            error!("Error parsing response body: {:?}", e);
                            false
                        }
                    }
                } else {
                    match filepath {
                        Some(filename) => { self.save_as_binary(response, filename); true },
                        None => { debug!("Got {} bytes of binary response, ignoring", response.content_length().unwrap_or(0)); false }
                    }
                }
            }, Err(e) => {
                error!("Error on api call: {:?}", e);
                false
            }
        }
    }

    fn save_as_json(&self, data: &str, filepath: String) {
        if data.len() == 0{
            return;
        }
        match File::create(&filepath) {
            Ok(file) => {
                let mut writer = BufWriter::new(file);
                let json_data: HashMap<String, serde_json::Value> = serde_json::from_str(data).unwrap();
                match serde_json::to_writer_pretty(&mut writer, &json_data) {
                    Ok(_) => {
                        match writer.flush() {
                            Ok(_) => {}, 
                            Err(e) => { error!("Error flushing writer: {}", e); }
                        }
                    }, Err(e) => { error!("Error writing json to buffer: {}", e); }
                }
            }, Err(e) => { error!("Unable to create file {}, error: {}", filepath, e); }
        }
    }

    fn save_as_binary(&self, mut response: Response, filepath: String){
        // .FIT files are saved as .ZIP files FYI
        let mut num_chunks = 0;
        match File::create(&filepath) {
            Ok(mut file) => {
                while let Ok(Some(chunk)) = self.runtime.block_on(response.chunk()) {
                    match file.write(&chunk){
                        Ok(_) => { num_chunks += 1; info!("Wrote chunk #{} to {}. Size: {}", num_chunks, &filepath, &chunk.len()); },
                        Err(e) => { error!("Error writing chunk #{} to {}, error: {}", num_chunks, &filepath, e); }
                    }
                }
            }, Err(e) => { error!("Unable to create file {}, error: {}", &filepath, e); }
        }
        if num_chunks == 0 {
            warn!("Didn't save any binary zip file data");
            return;
        }
        // now unzip the downloaded zip
        info!("Attempting to unzip files...");
        let file = File::open(&filepath).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let mut buffer: Vec<u8> = vec![];
            std::io::copy(&mut file, &mut buffer).expect("Unable to copy archive file contents to buffer :(");

            // get folder from filepath
            let new_path = Path::new(&filepath).parent().unwrap().join(&file.name());
            info!("Saving FIT file contents: {}", new_path.display());
            fs::write(new_path, &buffer).expect("Unable to write FIT file contents :(");
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
                if expiration < SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() {
                    warn!("Found garmin session token, but it expired. Need to re-authenticate");
                    return false;
                }
                info!("Successfully read garmin session token!");
                self.oauth_manager.oauth2_token.expires_at = expiration;
                self.oauth_manager.oauth2_token.oauth2_token.access_token = map["token"].to_string();
                return true;

            }, Err(e) => { info!("Unable opening garmin_client session file: {}", e); }
        }
        false
    }
    /// Saves the current access token if valid
    fn save_json_session(&self) {
        match File::create(&SESSION_FILE) {
            Ok(file) => {
                let mut writer = BufWriter::new(file);
                let json_data: HashMap<String, String> = HashMap::from([
                    (String::from("expires_at"), format!("{}", self.oauth_manager.oauth2_token.expires_at)),
                    (String::from("token"), String::from(&self.oauth_manager.oauth2_token.oauth2_token.access_token))
                ]);
                match serde_json::to_writer_pretty(&mut writer, &json_data) {
                    Ok(_) => {
                        match writer.flush() {
                            Ok(_) => { }, 
                            Err(e) => { error!("Error flushing writer: {}", e); }
                        }
                    }, Err(e) => { error!("Error writing json to buffer: {}", e); }
                }
            }, Err(e) => { error!("Unable to create file {}, error: {}", SESSION_FILE, e); }
        }
    }

    /// When specifying a JSON download in the api_request() function, this
    /// can be called to return that JSON text (use in lieu of saving JSON to file).
    pub fn get_last_resp_text(&self) -> &str {
        &self.last_api_resp_text
    }
}