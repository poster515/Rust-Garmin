
use std::collections::HashMap;
use log::{error, debug, warn};
use regex::Regex;
use reqwest::Error;
use reqwest::Url;
use reqwest::blocking::Client;
use reqwest::blocking::Response;
use reqwest::header::HeaderMap;

pub trait ClientTraits {
    fn login(&mut self, username: &str, password: &str) -> ();
    fn api_request(&mut self, endpoint: &str) -> ();
    fn get_session(&mut self, domain: &str, username: &str, password: &str) -> ();
}

// struct that knows how to navigate the auth flow for garmin connect api.
pub struct GarminClient {
    client: Client,
    auth_host: String,
    api_host: String,
    last_resp_url: String,
    last_resp_text: String,
    user_agent: HashMap<String, String>
}

impl GarminClient {
    // shamelessly adopted from:
    // https://github.com/cpfair/tapiriik/blob/master/tapiriik/services/GarminConnect/garminconnect.py#L10
    pub fn new() -> GarminClient {
        GarminClient {
            client: Client::builder().cookie_store(true).build().unwrap(),
            auth_host: String::from("https://sso.garmin.com/sso"),
            api_host: String::from("https://connectapi.garmin.com"),
            last_resp_url: String::new(),
            last_resp_text: String::new(),
            user_agent: HashMap::from([("User-Agent".to_owned(), "com.garmin.android.apps.connectmobile".to_owned())])
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

        let response = self.client.get(&url).send();
        match response {
            Ok(response) => {
                self.last_resp_url = response.url().to_string();
                self.last_resp_text = response.text().unwrap();
                true
            },
            Err(_) => false
        }
    }

    fn get_csrf_token(&mut self) -> bool {

        let url = self.build_singin_url();
        let mut headers = HeaderMap::new();
        headers.insert("referer", self.last_resp_url.as_str().parse().unwrap());
        
        // get csrf token
        let response = self.client.get(&url).headers(headers).send();
        match response {
            Ok(response) => {
                self.last_resp_url = response.url().to_string();
                self.last_resp_text = response.text().unwrap();
                true
            },
            Err(_) => false
        }
    }

    fn submit_login(&mut self, username: &str, password: &str, csrf_token: &str) -> bool {
        let url = self.build_singin_url();
        let mut headers = HeaderMap::new(); 
        headers.insert("referer", self.last_resp_url.as_str().parse().unwrap());

        let form = HashMap::from([
            ("username", String::from(username)),
            ("password", String::from(password)),
            ("embed", String::from("true")),
            ("_csrf", String::from(csrf_token))
        ]);
        
        let login_response = self.client.post(&url)
            .headers(headers)
            .form(&form)
            .send();

        match login_response {
            Ok(response) => {
                self.last_resp_url = response.url().to_string();
                self.last_resp_text = response.text().unwrap();
                true
            },
            Err(_) => false
        }
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

}

#[allow(unused_variables)]
impl ClientTraits for GarminClient {
    fn login(&mut self, username: &str, password: &str) -> () {

        // set cookies
        if !self.set_cookie() {
            return
        }

        // get csrf token
        if !self.get_csrf_token() {
            return
        }
        
        let csrf_token = self.parse_csrf_token(&self.last_resp_text);
        
        if csrf_token.len() == 0 {
            return
        }

        // Submit login form with email and password
        self.submit_login(username, password, &csrf_token);
        let title = self.parse_title(&self.last_resp_text);
        if title.len() == 0 {
            return
        }

        let ticket = self.parse_ticket(&self.last_resp_text);
        if ticket.len() == 0 {
            return;
        }
    }

    fn api_request(&mut self, endpoint: &str) -> () {
        // use for actual application data downloads

        // TODO: give filename for saving json data

    }

    fn get_session(&mut self, domain: &str, username: &str, password: &str) -> () {
        // TODO: cache credentials and store them

        let mut data = HashMap::new();
        data.insert("username", username);
        data.insert("password", password);
        data.insert("_eventId", "submit");
        data.insert("embed", "true");
        // data.insert("displayNameRequired", "false");

        let mut params = HashMap::new();
        params.insert("service", "https://connect.garmin.com/modern");
        // params.insert("redirectAfterAccountLoginUrl", "http://connect.garmin.com/modern");
        // params.insert("redirectAfterAccountCreationUrl", "http://connect.garmin.com/modern");
        // params.insert("webhost", "olaxpw-connect00.garmin.com");
        params.insert("clientId", "GarminConnect");
        params.insert("gauthHost", "https://sso.garmin.com/sso");
        // params.insert("rememberMeShown", "true");
        // params.insert("rememberMeChecked", "false");
        params.insert("consumeServiceTicket", "false");
        // params.insert("id", "gauth-widget");
        // params.insert("embedWidget", "false");
        // params.insert("cssUrl", "https://static.garmincdn.com/com.garmin.connect/ui/src-css/gauth-custom.css");
        // params.insert("source", "http://connect.garmin.com/en-US/signin");
        // params.insert("createAccountShown", "true");
        // params.insert("openCreateAccount", "false");
        // params.insert("usernameShown", "true");
        // params.insert("displayNameShown", "false");
        // params.insert("initialFocus", "true");
        // params.insert("locale", "en");

        
        let res: Response = self.client.get(domain)
            .query(&params)
            .send()
            .unwrap();

        if !res.status().is_success() {
            error!("Got non success status code: {}", res.status());
        }
    }
}