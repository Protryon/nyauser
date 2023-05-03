use std::net::SocketAddr;

use indexmap::IndexMap;
use serde::Deserialize;

use crate::{
    db::{Profile, Series},
    search::SearchConfig,
    sink::SinkConfig,
    source::SourceConfig,
};

#[derive(Deserialize)]
pub struct Config {
    pub bind: SocketAddr,
    pub rpc_username: String,
    pub rpc_password: String,
    pub sinks: IndexMap<String, SinkConfig>,
    pub sources: IndexMap<String, SourceConfig>,
    pub search: SearchConfig,
    pub db_file: String,
    pub profiles: Vec<Profile>,
    pub series: Vec<Series>,
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
