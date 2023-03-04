use std::collections::HashMap;

use serde::Deserialize;

use crate::{transmission::TransmissionConfig, nyaa::NyaaConfig, search::SearchConfig, profile::ProfileConfig, series::SeriesConfig};


#[derive(Deserialize)]
pub struct Config {
    pub transmission: TransmissionConfig,
    pub nyaa: NyaaConfig,
    pub search: SearchConfig,
    pub profiles: HashMap<String, ProfileConfig>,
    pub series: HashMap<String, SeriesConfig>,
    pub db_file: String,
}

lazy_static::lazy_static! {
    pub static ref CONFIG: Config = {
        let mut path = std::env::var("NYAUSER_CONFIG").unwrap_or_default();
        if path.is_empty() {
            path = "config.yml".to_string();
        }
        let raw = std::fs::read_to_string(path).expect("failed to read config");
        serde_yaml::from_str(&raw).expect("failed to parse config")
    };
}