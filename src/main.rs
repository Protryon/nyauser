use config::CONFIG;
use log::LevelFilter;
use nyaa::NyaaClient;
use search::Searcher;
use transmission::TransmissionClient;
use clap::Parser;

mod config;
mod transmission;
mod nyaa;
mod search;
mod profile;
mod series;


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
        .parse_env(env_logger::Env::default().default_filter_or(if args.verbose { "debug" } else { "info" }))
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
    if args.wipe_nonexistant {
        search::wipe_nonexistant(&db).await;
        return;
    }

    let sink = TransmissionClient::new();
    let source = NyaaClient::new();
    let mut searcher = Searcher::new(db, source, sink).expect("failed to init searcher");
    if args.clean {
        searcher.clean().await.expect("clean failed");
        return;
    }

    info!("running searcher");
    searcher.run().await;
}
