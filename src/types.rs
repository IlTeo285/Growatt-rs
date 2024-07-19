use chrono::{Local, Utc, DateTime};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(thiserror::Error, Debug)]
pub enum RequestError {
    
    #[error("failed to login, unable to extract credentials {0}")]
    LoginCredentialExtraction(#[from] reqwest::header::InvalidHeaderValue),

    #[error("failed to login")]
    LoginFailed,

    #[error("unable to perform the reqest {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("unable to decode response {0}")]
    MalformedResponse(#[from] serde_json::Error),

    #[error("unable to decode response")]
    GenericError,
}

pub type GrowattResult<T> = Result<T, RequestError>;

pub(crate) mod utils {

    use chrono::{DateTime, Utc, NaiveDateTime};
    use serde::de::{self, Deserialize, Deserializer};
    use std::fmt::Display;
    use std::str::FromStr;

    const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

    pub fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        T::from_str(&s).map_err(de::Error::custom)
    }

    pub fn from_date<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?;
        Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
    }
}

#[derive(Copy, Clone, Serialize)]
pub struct When(i64);
impl Default for When {
    fn default() -> Self {
        Self(Utc::now().timestamp_nanos_opt().unwrap_or_default())
    }
}

impl ToString for When {
    fn to_string(&self) -> String {
        let dt = chrono::DateTime::from_timestamp_nanos(self.0);
        let now_local: DateTime<Local> = dt.with_timezone(&Local);
        format!("{}", now_local.format("%d/%m/%Y %H:%M:%S"))
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

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct MixTotalData {
    #[serde(skip_deserializing)]
    pub when: When,

    #[serde(deserialize_with = "utils::from_str")]
    eselfToday: f32,

    #[serde(deserialize_with = "utils::from_str")]
    gridPowerTotal: f32,

    #[serde(deserialize_with = "utils::from_str")]
    eselfTotal: f32,

    #[serde(deserialize_with = "utils::from_str")]
    elocalLoadToday: f32,

    #[serde(deserialize_with = "utils::from_str")]
    gridPowerToday: f32,

    #[serde(deserialize_with = "utils::from_str")]
    elocalLoadTotal: f32,
    
    #[serde(deserialize_with = "utils::from_str")]
    eexTotal: f32,
    
    #[serde(deserialize_with = "utils::from_str")]
    photovoltaicRevenueToday: f32,
    
    #[serde(deserialize_with = "utils::from_str")]
    eexToday: f32,

    #[serde(deserialize_with = "utils::from_str")]
    etoGridToday: f32,

    #[serde(deserialize_with = "utils::from_str")]
    edischarge1Total: f32,

    #[serde(deserialize_with = "utils::from_str")]
    photovoltaicRevenueTotal: f32,

    unit: String,

    #[serde(deserialize_with = "utils::from_str")]
    edischarge1Today: f32,

    #[serde(deserialize_with = "utils::from_str")]
    epvToday: f32,

    #[serde(deserialize_with = "utils::from_str")]
    epvTotal: f32,

    #[serde(deserialize_with = "utils::from_str")]
    etogridTotal: f32,
}

impl MixTotalData {
    pub fn get_today_data(&self) -> TodayMixData {
        TodayMixData{
            when: self.when,
            eself: self.eselfToday,
            elocalLoad: self.elocalLoadToday,
            gridPower: self.gridPowerToday,
            photovoltaicRevenue: self.photovoltaicRevenueToday,
            eex: self.eexToday,
            etoGrid: self.etoGridToday,
            edischarge1: self.edischarge1Today,
            epv: self.epvToday, 
        }
    }

    pub fn get_total_data(&self) -> TotalMixData {
        TotalMixData{
            when: self.when,
            eself: self.eselfTotal,
            elocalLoad: self.elocalLoadTotal,
            gridPower: self.gridPowerTotal,
            photovoltaicRevenue: self.photovoltaicRevenueTotal,
            eex: self.eexTotal,
            etoGrid: self.etogridTotal,
            edischarge1: self.edischarge1Total,
            epv: self.epvTotal, 
        }
    }
}

#[derive(Debug)]
#[allow(non_snake_case)]
#[allow(dead_code)]
pub struct TodayMixData {
    pub when: When,
    pub eself: f32,
    pub elocalLoad: f32,
    pub gridPower: f32,
    pub photovoltaicRevenue: f32,
    pub eex: f32,
    pub etoGrid: f32,
    pub edischarge1: f32,
    pub epv: f32,
}

#[derive(Debug)]
#[allow(non_snake_case)]
#[allow(dead_code)]
pub struct TotalMixData {
    pub when: When,
    pub gridPower: f32,
    pub eself: f32,
    pub elocalLoad: f32,
    pub eex: f32,
    pub edischarge1: f32,
    pub photovoltaicRevenue: f32,
    pub epv: f32,
    pub etoGrid: f32,
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Data {
    #[serde(deserialize_with = "utils::from_str")]
    ptoStatus: u32,

    #[serde(deserialize_with = "utils::from_date")]
    timeServer: DateTime<Utc>,

    accountName: String,
    
    #[serde(deserialize_with = "utils::from_str")]
    timezone: f32,

    #[serde(deserialize_with = "utils::from_str")]
    bctMode: u32,

    #[serde(deserialize_with = "utils::from_str")]
    bdcStatus: u32,

    #[serde(deserialize_with = "utils::from_str")]
    eMonth: f32,

    #[serde(deserialize_with = "utils::from_str")]
    dtc: f32,

    #[serde(deserialize_with = "utils::from_str")]
    pac: f32,

    #[serde(deserialize_with = "utils::from_str")]
    batSysRateEnergy: f32,

    datalogSn: String,
    alias: String,
    sn: String,

    #[serde(deserialize_with = "utils::from_str")]
    deviceType: u32,

    plantId: String,
    deviceTypeName: String,

    #[serde(deserialize_with = "utils::from_str")]
    nominalPower: f32,

    #[serde(deserialize_with = "utils::from_str")]
    eToday: f32,

    datalogTypeTest: String,
    
    #[serde(deserialize_with = "utils::from_str")]
    eTotal: f32,


    showDeviceModel: String,
    location: String,
    deviceModel: String,
    plantName: String,
    
    #[serde(deserialize_with = "utils::from_str")]
    status: u32,

    #[serde(deserialize_with = "utils::from_date")]
    lastUpdateTime: DateTime<Utc>,
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct DeviceList {
    currPage: u32,
    pages: u32,
    pageSize: u32,
    count: u32,
    ind: u32,
    datas: Vec<Data>,
    notPager: bool,
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct GrowattResponse<T: Debug> {
    result: i32,
    obj: T,
}

impl<T> GrowattResponse<T> where T: Debug {
    pub fn is_ok(&self) -> bool {
        return self.result > 0;
    }

    pub fn into_inner(self) -> T {
        self.obj
    }
}


#[cfg(test)]
mod when_test {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn from() {
        let wh = When(1234i64);
        assert_eq!(Into::<i64>::into(wh), 1234i64);
    }

    #[test]
    fn to_string() {
        let date_time: DateTime<Local> = Local.with_ymd_and_hms(2017, 04, 02, 12, 50, 32).unwrap();
        let wh = When(date_time.timestamp_nanos_opt().unwrap());
        let formatted = format!("{}", wh.to_string());
        assert_eq!(formatted, "02/04/2017 12:50:32");
    }
}