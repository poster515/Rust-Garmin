
use std::collections::HashMap;
use config::Config;
use log::error;
use reqwest::blocking::Client;
use reqwest::blocking::Response;

trait ClientTraits {
    fn login(&mut self) -> ();
    fn request(&mut self, subdomain: &str, endpoint: &str) -> ();
    fn get_session(&mut self, domain: &str, username: &str, password: &str) -> ();
}


pub struct GarminClient {
    client: Client,
    garmin_signin_headers: &'static str
}

impl GarminClient {
    // shamelessly adopted from:
    // https://github.com/cpfair/tapiriik/blob/master/tapiriik/services/GarminConnect/garminconnect.py#L10
    pub fn new(config: &Config) -> GarminClient {
        GarminClient {
            client: Client::builder().cookie_store(true).build().unwrap(),
            garmin_signin_headers: "https://sso.garmin.com"
        }
    }
}

#[allow(unused_variables)]
impl ClientTraits for GarminClient {
    fn login(&mut self) -> () {

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