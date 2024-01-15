
use std::collections::HashMap;
use log::{error, debug, warn, info};
use regex::Regex;
use reqwest::Client;
use reqwest::header::HeaderMap;

mod auth;

pub trait ClientTraits {
    fn login(&mut self, username: &str, password: &str) -> ();
    fn api_request(&mut self, endpoint: &str) -> ();
}

// struct that knows how to navigate the auth flow for garmin connect api.
#[allow(dead_code)]
pub struct GarminClient {
    client: Client,
    auth_host: String,
    last_sso_resp_url: String,
    last_sso_resp_text: String,
    last_api_resp_url: String,
    last_api_resp_text: String,
    user_agent: HashMap<String, String>,
    oauth_manager: auth::GaminOAuthManager
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
            user_agent: HashMap::from([("User-Agent".to_owned(), "com.garmin.android.apps.connectmobile".to_owned())]),
            oauth_manager: auth::GaminOAuthManager::new()
        }
    }

    fn build_singin_url(&self) -> String {
        let mut sso_embed = String::from(&self.auth_host);
        sso_embed.push_str("/embed");

        let mut ub = url_builder::URLBuilder::new();
        ub.set_protocol("https")
            .set_host("sso.garmin.com")
            .add_route("sso")
            .add_route("signin")
            .add_param("id", "gauth-widget")
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

        let rt = tokio::runtime::Runtime::new().unwrap();
        let future = rt.block_on({
            self.client.get(&url).send()
        });

        let response = future.unwrap();
        self.last_sso_resp_url = response.url().to_string();

        let get_body_future = rt.block_on({
            response.text()
        });

        self.last_sso_resp_text = get_body_future.unwrap();
        true
    }

    fn get_csrf_token(&mut self) -> bool {

        let url = self.build_singin_url();
        let mut headers = HeaderMap::new();
        headers.insert("referer", self.last_sso_resp_url.as_str().parse().unwrap());

        let rt = tokio::runtime::Runtime::new().unwrap();
        let future = rt.block_on({
            self.client.get(&url).headers(headers).send()
        });

        let response = future.unwrap();
        self.last_sso_resp_url = response.url().to_string();

        let get_body_future = rt.block_on({
            response.text()
        });

        self.last_sso_resp_text = get_body_future.unwrap();
        true
    }

    fn submit_login(&mut self, username: &str, password: &str, csrf_token: &str) -> bool {
        let url = self.build_singin_url();
        let mut headers = HeaderMap::new(); 
        headers.insert("referer", self.last_sso_resp_url.as_str().parse().unwrap());

        let form = HashMap::from([
            ("username", String::from(username)),
            ("password", String::from(password)),
            ("embed", String::from("true")),
            ("_csrf", String::from(csrf_token))
        ]);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let future = rt.block_on({
            self.client.post(&url)
                .headers(headers)
                .form(&form)
                .send()
        });

        let response = future.unwrap();
        self.last_sso_resp_url = response.url().to_string();

        let get_body_future = rt.block_on({
            response.text()
        });

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
                debug!("Got successful login!");
                return String::from(title);
            } else if title == "GARMIN Authentication Application" {
                error!("Got unsuccessful login :( check your credentials?");
            } else {
                warn!("Unsure how to process login response {}", title);
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

    pub fn login(&mut self, username: &str, password: &str) -> () {

        // set cookies
        if !self.set_cookie() {
            return
        }

        // get csrf token
        if !self.get_csrf_token() {
            return
        }
        
        let csrf_token: String = self.parse_csrf_token(&self.last_sso_resp_text);
        
        if csrf_token.len() == 0 {
            return
        }

        // Submit login form with email and password
        self.submit_login(username, password, &csrf_token);
        let title = self.parse_title(&self.last_sso_resp_text);
        if title.len() == 0 {
            return
        }

        let ticket = self.parse_ticket(&self.last_sso_resp_text);
        if ticket.len() == 0 {
            return;
        }

        let _oauth1 = self.set_oauth1_token(&ticket);
        let _oauth2 = self.set_oauth2_token();
    }

    fn set_oauth1_token(&mut self, ticket: &str) -> bool {
        let oauth1_token: String = self.oauth_manager.set_oauth1_token(ticket, self.client.clone()).unwrap();
        info!("Got oauth1 token: {}", oauth1_token);
        true
    }

    fn set_oauth2_token(&mut self) -> bool {
        let oauth2_token: String = self.oauth_manager.set_oauth2_token(self.client.clone()).unwrap();
        info!("Got oauth2 token: {}", oauth2_token);
        true
    }

    pub fn api_request(&mut self, endpoint: &str) -> () {
        // use for actual application data downloads

        // TODO: give filename for saving json data
        let url = self.build_api_url(endpoint).build();

        if self.oauth_manager.get_oauth2_token().is_expired() {
            info!("====================================================");
            info!("ConnectAPI refreshing OAuth2.0 token...");
            info!("====================================================");
            self.set_oauth2_token();
        }

        let access_token: String = String::from(&self.oauth_manager.get_oauth2_token().oauth2_token.access_token);

        debug!("====================================================");
        debug!("ConnectAPI requesting from: {}", &url);
        debug!("====================================================");

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", access_token.as_str().parse().unwrap());

        let rt = tokio::runtime::Runtime::new().unwrap();
        let future = rt.block_on({
            self.client.get(url).headers(headers).send()
        });

        let response = future.unwrap();
        self.last_api_resp_url = response.url().to_string();

        let get_body_future = rt.block_on({
            response.text()
        });

        self.last_api_resp_text = get_body_future.unwrap();
        debug!("Got api response: {}", &self.last_api_resp_text);
    }
}