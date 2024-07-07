use chrono::offset::Utc;
use reqwest::{
    header::{self, HeaderValue},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;

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
