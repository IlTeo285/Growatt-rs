use chrono::offset::Utc;
use regex::Regex;
use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use reqwest::{header, Client};

pub(crate) mod utils {

    use serde::de::{self, Deserialize, Deserializer};
    use std::fmt::Display;
    use std::str::FromStr;

    pub fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        T::from_str(&s).map_err(de::Error::custom)
    }
}

pub struct GrowattServer {
    server_url: String,
    client: Client,
    cookie: header::HeaderMap,
    referer: String,
}

impl Default for GrowattServer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone, Serialize)]
pub struct When(i64);
impl Default for When {
    fn default() -> Self {
        Self(Utc::now().timestamp_nanos_opt().unwrap())
    }
}

impl Debug for When {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ns", self.0)
    }
}

impl From<When> for i64 {
    fn from(lhs: When) -> i64 {
        lhs.0
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, Copy)]
pub struct MixStatus {
    #[serde(skip_deserializing)]
    pub when: When,

    #[serde(rename = "chargePower")]
    pub power_battery_charge: f32,

    #[serde(deserialize_with = "utils::from_str")]
    #[serde(rename = "SOC")]
    pub soc: u32,

    #[serde(rename = "pLocalLoad")]
    pub power_to_load: f32,

    #[serde(deserialize_with = "utils::from_str")]
    #[serde(rename = "pPv1")]
    pub power_from_photovoltaic_1: f32,

    #[serde(rename = "pactogrid")]
    pub power_to_grid: f32,

    #[serde(rename = "pactouser")]
    pub power_to_user: f32,

    #[serde(rename = "pdisCharge1")]
    pub power_battery_discharge: f32,

    #[serde(rename = "vAc1")]
    #[serde(deserialize_with = "utils::from_str")]
    pub voltage_grid: f32,

    #[serde(rename = "vBat")]
    #[serde(deserialize_with = "utils::from_str")]
    pub voltage_battery: f32,

    #[serde(rename = "vPv1")]
    #[serde(deserialize_with = "utils::from_str")]
    pub voltage_photovoltaic_1: f32,
}

impl GrowattServer {
    pub fn new() -> Self {
        Self {
            server_url: "https://server.growatt.com/".to_owned(),
            referer: "".to_owned(),
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

        self.cookie.clear();
        for el in res.headers().get_all("set-cookie") {
            let current_cookie = el.to_str()?;
            log::trace!("using cookie {}", current_cookie);

            if let Some(caps) = re_session.captures(current_cookie) {
                self.referer = format!(
                    "https://server.growatt.com/index;jsessionid={}",
                    caps[1].to_owned()
                );
                self.cookie.append("cookie", el.clone());
            }

            if let Some(_) = se_session.captures(current_cookie) {
                self.cookie.append("cookie", el.clone());
            }
        }

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

        let mut hm = header::HeaderMap::new();
        hm.insert("Referer", self.referer.parse().unwrap());

        let res = self
            .client
            .post(url)
            .headers(self.cookie.clone())
            .headers(hm)
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

        let mut hm = header::HeaderMap::new();
        hm.insert("Referer", self.referer.parse().unwrap());

        let res = self
            .client
            .post(url)
            .headers(self.cookie.clone())
            .headers(hm)
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
    #[actix_rt::test]
    async fn login() {
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


        let res = client
            .mix_system_status(&mix_id, &plant_id)
            .await;

        assert_eq!(res.is_ok(), true);
    }
}
