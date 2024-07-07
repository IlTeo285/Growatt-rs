pub mod types;

use regex::Regex;
use types::{RequestError, GrowattResult};
use std::collections::HashMap;

use serde_json::Value;

use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client, ClientBuilder,
};

const BASE_SERVER_URL: &str = "https://server.growatt.com/";

macro_rules! url {
    ($page:expr) => {
            format!("{}{}", BASE_SERVER_URL, $page)
    };
}

pub struct GrowattServer {
    client: Client,
    cookie: Option<header::HeaderMap>,
}

impl Default for GrowattServer {
    fn default() -> Self {
        Self::new()
    }
}

impl GrowattServer {
    pub fn new() -> Self {

        let mut headers = header::HeaderMap::new();
        headers.insert("User-Agent", header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.0.0 Safari/537.36-11"));
        headers.insert("Connection", header::HeaderValue::from_static("keep-alive"));

        let client_builder = ClientBuilder::new()
            .default_headers(headers);

        Self {
            client: client_builder.build().expect("unable to build client"),
            cookie: None,
        }
    }

    fn check_res(body: String) -> bool {
        let parse_check = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| v.get("result").and_then(|value| value.as_i64()))
            .map(|num| if num == 0 { false } else { true })
            .unwrap_or(false);

        parse_check
    }

    pub async fn login(
        &mut self,
        username: &str,
        password: &str,
    ) -> GrowattResult {

        let payload: HashMap<&str, &str> =
            HashMap::from([("account", username), ("password", password)]);

        let res = self
            .client
            .post(url!("login"))
            .form(&payload)
            .send()
            .await?;

        log::trace!("login request with status {}", res.status().as_str());

        let re_session = Regex::new(r"JSESSIONID=([^;]+)").unwrap();
        let se_session = Regex::new(r"SERVERID=").unwrap();

        // Build cookie for reply

        let cookie = res
            .headers()
            .get_all("Set-Cookie")
            .iter()
            .filter(|element| {
                let current_cookie = element.to_str().unwrap_or_default();
                log::trace!("response set-cookie: {}", current_cookie);

                re_session.captures(current_cookie).is_some()
                    || se_session.captures(current_cookie).is_some()
            })
            .map(|el| el.to_str().unwrap_or_default())
            .collect::<Vec<&str>>()
            .join(";");
        log::trace!("Cookie: {:?}", cookie);

        let cookie = HeaderValue::from_str(&cookie)?;
        let mut h_map = HeaderMap::new();
        h_map.append("Cookie", cookie);

        self.cookie = Some(h_map);

        let body = res.text().await?;

        if Self::check_res(body.clone()) == false {
            Err(RequestError::LoginFailed)
        } else {
            Ok(body)
        }
    }

    pub async fn mix_system_status(
        &self,
        mix_id: &str,
        plant_id: &str,
    ) -> GrowattResult {

        let api = format!("panel/mix/getMIXStatusData?plantId={}", plant_id);

        let mut payload = HashMap::new();
        payload.insert("mixSn", mix_id);

        let mut reqest = self
            .client
            .post(url!(api))
            .form(&payload);

        if let Some(cred) = self.cookie.as_ref() {
            reqest = reqest.headers(cred.clone())
        }

        let res  = reqest.send().await?;

        log::trace!(
            "mix_system_status request with status {}",
            res.status().as_str()
        );

        let content = res.text().await?;

        //Strip off unusefull part
        let v =
            serde_json::from_str(&content).and_then(|v: Value| serde_json::to_string(&v["obj"]))?;
        Ok(v)
    }

    pub async fn device_list_by_plant(
        &self,
        plant_id: &str,
    ) -> GrowattResult {
        let api = format!(
            "panel/getDevicesByPlantList?plantId={}&currPage=1",
            plant_id
        );

        let mut reqest = self
            .client
            .post(url!(api));

        if let Some(cred) = self.cookie.as_ref() {
            reqest = reqest.headers(cred.clone())
        }

        let res  = reqest.send().await?;

        log::trace!("plant_list request with status {}", res.status().as_str());

        let content = res.text().await?;
        if Self::check_res(content.clone()) == false {
            Err(RequestError::GenericError)
        } else {
            Ok(content)
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::*;
    use log::info;

    fn init() {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Trace)
            .try_init();
    }

    #[actix_rt::test]
    async fn login() {
        init();
        info!("Start Login test");

        let username = std::env::var("GROWATT_TESTS_USERNAME").unwrap();
        let password = std::env::var("GROWATT_TESTS_PASSWORD").unwrap();

        let mut client = GrowattServer::new();
        assert!(client.login(&username, &password).await.is_ok());
    }

    #[actix_rt::test]
    async fn login_wrong_credential() {
        let username = "one".to_owned();
        let password = "two".to_owned();

        let mut client = GrowattServer::new();
        assert_eq!(client.login(&username, &password).await.is_err(), false);
    }

    #[actix_rt::test]
    async fn get_mix_data() -> Result<(), RequestError> {
        let username = std::env::var("GROWATT_TESTS_USERNAME").unwrap();
        let password = std::env::var("GROWATT_TESTS_PASSWORD").unwrap();
        let plant_id = std::env::var("GROWATT_TESTS_PLANTID").unwrap();
        let mix_id = std::env::var("GROWATT_TESTS_MIXID").unwrap();

        let mut client = GrowattServer::new();
        client.login(&username, &password).await?;

        client.device_list_by_plant(&plant_id).await?;
        client.mix_system_status(&mix_id, &plant_id).await?;

        Ok(())
    }
}
