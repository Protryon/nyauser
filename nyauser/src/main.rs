use std::sync::Arc;

use clap::Parser;
use config::CONFIG;
use log::LevelFilter;
use search::Searcher;
use sink::{Sink, TransmissionClient};
use source::{NyaaClient, Source};

use crate::{api::AppState, db::Database, sink::SinkConfig, source::SourceConfig};

mod api;
mod config;
mod db;
mod search;
mod sink;
mod source;

/// Simplified torrent puller
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Dumps the contents of the database and exits
    #[clap(short, long)]
    dump: bool,

    /// Cleans the contents of the database and exits
    #[clap(short, long)]
    clean: bool,

    /// Wipes all finished torrent information for files that no longer exist
    #[clap(long)]
    wipe_nonexistant: bool,

    /// Increases log level
    #[clap(short, long)]
    verbose: bool,
}

#[macro_use]
extern crate anyhow;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    env_logger::Builder::new()
        .filter_module("transmission_rpc", LevelFilter::Warn)
        .parse_env(
            env_logger::Env::default().default_filter_or(if args.verbose {
                "debug"
            } else {
                "info"
            }),
        )
        .init();

    let db = sled::open(&*CONFIG.db_file).expect("sled failed to init db");
    if args.dump {
        println!("dumping");
        for (key, value) in db.iter().map(|x| x.unwrap()) {
            let key = std::str::from_utf8(&key[..]).unwrap();
            let value = std::str::from_utf8(&value[..]).unwrap();
            println!("{} = {}", key, value);
        }
        return;
    }
    let db = Arc::new(Database::new(db));
    if args.wipe_nonexistant {
        search::wipe_nonexistant(&db).expect("wipe_nonexistant failed");
        return;
    }

    let Some(source_config) = CONFIG.sources.get(&CONFIG.search.source) else {
        error!("invalid source {}, not found", CONFIG.search.source);
        std::process::exit(1);
    };
    let Some(sink_config) = CONFIG.sinks.get(&CONFIG.search.sink) else {
        error!("invalid sink {}, not found", CONFIG.search.sink);
        std::process::exit(1);
    };

    let sink: Box<dyn Sink + Send + Sync> = match sink_config {
        SinkConfig::Transmission(config) => Box::new(TransmissionClient::new(config.clone())),
    };
    let source: Box<dyn Source + Send + Sync> = match source_config {
        SourceConfig::Nyaa(config) => Box::new(NyaaClient::new(config.clone())),
    };
    let mut searcher = Searcher::new(db.clone(), source, sink, CONFIG.search.clone())
        .expect("failed to init searcher");
    if args.clean {
        searcher.clean().await.expect("clean failed");
        return;
    }
    for profile in &CONFIG.profiles {
        db.save_profile(profile).expect("Failed to load profile");
    }
    for series in &CONFIG.series {
        db.save_series(series).expect("Failed to load series");
    }

    api::spawn_api_server(AppState { database: db });

    info!("running searcher");
    searcher.run().await;
}
