
use std::collections::HashMap;
use log::{error, debug, warn};
use regex::Regex;
use reqwest::Url;
use reqwest::blocking::Client;
use reqwest::blocking::Response;
use reqwest::header::HeaderMap;

pub trait ClientTraits {
    fn login(&mut self, username: &str, password: &str) -> ();
    fn request(&mut self, subdomain: &str, endpoint: &str) -> ();
    fn get_session(&mut self, domain: &str, username: &str, password: &str) -> ();
}

// struct that knows how to navigate the auth flow for garmin connect api.
pub struct GarminClient {
    client: Client,
    auth_host: String
}

impl GarminClient {
    // shamelessly adopted from:
    // https://github.com/cpfair/tapiriik/blob/master/tapiriik/services/GarminConnect/garminconnect.py#L10
    pub fn new() -> GarminClient {
        GarminClient {
            client: Client::builder().cookie_store(true).build().unwrap(),
            auth_host: String::from("https://sso.garmin.com/sso"),
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

    fn set_cookie(&mut self) -> Result<Response, reqwest::Error> {
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

        self.client.get(&url).send()
    }

    fn get_csrf_token(&mut self, referer_url: &Url) -> Result<Response, reqwest::Error> {

        let url = self.build_singin_url();
        let mut headers = HeaderMap::new();
        headers.insert("referer", referer_url.as_str().parse().unwrap());
        
        // get csrf token
        self.client.get(&url)
            .headers(headers)
            .send()
    }

    fn submit_login(&self, username: &str, password: &str, referer_url: &Url, csrf_token: &str) -> () {
        let url = self.build_singin_url();
        let mut headers = HeaderMap::new(); 
        headers.insert("referer", referer_url.as_str().parse().unwrap());

        let form = HashMap::from([
            ("username", String::from(username)),
            ("password", String::from(password)),
            ("embed", String::from("true")),
            ("_csrf", String::from(csrf_token))
        ]);
        
        let login_response: Response = self.client.post(&url)
            .headers(headers)
            .form(&form)
            .send()
            .unwrap();

        let login_html: String = login_response.text().unwrap();

        self.parse_title(&login_html);

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
        for (_, [csrf]) in re.captures_iter(&response_html).map(|c| c.extract()) {

            debug!("====================================================");
            if csrf == "Success" {
                debug!("Got successful login!");
            } else if csrf == "GARMIN Authentication Application" {
                error!("Got unsuccessful login :( check your credentials?");
            } else {
                warn!("Unsure how to process login response {}", csrf);
            }
            debug!("====================================================");
            return String::from(csrf);
        }
        error!("====================================================");
        error!("Unable to find title in body: {}", response_html);
        error!("====================================================");
        String::new()
    }

}

#[allow(unused_variables)]
impl ClientTraits for GarminClient {
    fn login(&mut self, username: &str, password: &str) -> () {

        // set cookies
        let auth_response: Response = self.set_cookie().unwrap();
        let referer_url: &Url = auth_response.url();

        // get csrf token
        let csrf_response: Response = self.get_csrf_token(referer_url).unwrap();
        let referer_url: Url = csrf_response.url().clone();
        let csrf_html: String = csrf_response.text().unwrap();
        let csrf_token = self.parse_csrf_token(&csrf_html);
        
        if csrf_token.len() == 0 {
            return
        }

        // Submit login form with email and password
        self.submit_login(username, password, &referer_url, &csrf_token);
    }

    fn request(&mut self, subdomain: &str, endpoint: &str) -> () {

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