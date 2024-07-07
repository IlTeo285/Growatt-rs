pub mod types;

use regex::Regex;
use std::collections::HashMap;

use serde_json::Value;

use reqwest::{
    header::{self, HeaderValue},
    Client,
};

pub struct GrowattServer {
    server_url: String,
    client: Client,
    cookie: header::HeaderMap,
}

impl Default for GrowattServer {
    fn default() -> Self {
        Self::new()
    }
}

impl GrowattServer {
    pub fn new() -> Self {
        Self {
            server_url: "https://server.growatt.com/".to_owned(),
            client: Client::builder().build().unwrap(),
            cookie: header::HeaderMap::new(),
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

    fn get_url(&self, page: &str) -> String {
        let mut ret = self.server_url.clone();
        ret.push_str(page);
        ret
    }

    pub async fn login(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = self.get_url("login");

        let mut headers = header::HeaderMap::new();
        headers.insert("User-Agent", header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.0.0 Safari/537.36-11"));
        headers.insert("Connection", header::HeaderValue::from_static("keep-alive"));

        let payload: HashMap<&str, &str> =
            HashMap::from([("account", username), ("password", password)]);

        let res = self
            .client
            .post(url)
            .headers(headers)
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
        self.cookie.append("Cookie", cookie);

        let body = res.text().await?;

        if Self::check_res(body.clone()) == false {
            Err(
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing success field")
                    .into(),
            )
        } else {
            Ok(body)
        }
    }

    pub async fn mix_system_status(
        &self,
        mix_id: &str,
        plant_id: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("panel/mix/getMIXStatusData?plantId={}", plant_id);
        let url = self.get_url(&url);

        let mut payload = HashMap::new();
        payload.insert("mixSn", mix_id);

        let res = self
            .client
            .post(url)
            .headers(self.cookie.clone())
            .form(&payload)
            .send()
            .await?;

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
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!(
            "panel/getDevicesByPlantList?plantId={}&currPage=1",
            plant_id
        );
        let url = self.get_url(&url);

        let res = self
            .client
            .post(url)
            .headers(self.cookie.clone())
            .send()
            .await?;

        log::trace!("plant_list request with status {}", res.status().as_str());

        let content = res.text().await?;
        if Self::check_res(content.clone()) == false {
            Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Succeed false").into())
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
    async fn get_mix_data() {
        let username = std::env::var("GROWATT_TESTS_USERNAME").unwrap();
        let password = std::env::var("GROWATT_TESTS_PASSWORD").unwrap();
        let plant_id = std::env::var("GROWATT_TESTS_PLANTID").unwrap();
        let mix_id = std::env::var("GROWATT_TESTS_MIXID").unwrap();

        let mut client = GrowattServer::new();
        client.login(&username, &password).await.unwrap();

        let res = client.device_list_by_plant(&plant_id).await;

        let res = client.mix_system_status(&mix_id, &plant_id).await;

        assert_eq!(res.is_ok(), true);
    }
}
