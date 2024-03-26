use std::iter::once;

use anyhow::{anyhow, bail, Result};
use clap::{Parser, Subcommand};
use cli_table::{print_stdout, Table, WithTitle};
use nyauser_types::{
    Episode, Profile, PullEntryFilter, PullEntryNamed, PullState, RegexWrapper, Series,
    SeriesStatus,
};
use regex::Regex;
use reqwest::{Client, Method, RequestBuilder, StatusCode, Url};

/// Simplified torrent puller
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// If set, overrides default and env var lookup (NYAUSER_API) for nyauser API server (default is http://localhost:8000/)
    #[clap(short, long)]
    api: Option<Url>,
    /// If set, overrides default and env var lookup (NYAUSER_USER) for nyauser API username (default is nyauser)
    #[clap(short, long)]
    user: Option<String>,
    /// If set, overrides default and env var lookup (NYAUSER_PASS) for nyauser API password (default is nyauser)
    #[clap(short, long)]
    pass: Option<String>,
    /// Increases log level
    #[clap(short, long)]
    verbose: bool,

    #[clap(subcommand)]
    mode: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    Profile {
        #[clap(subcommand)]
        mode: ProfileMode,
    },
    Pull {
        #[clap(subcommand)]
        mode: PullMode,
    },
    Series {
        #[clap(subcommand)]
        mode: SeriesMode,
    },
    /// Procs an immediate scan of download clients for completed downloads
    Scan,
    /// Procs an immediate search for new torrents
    Search,
    /// Wipes all knowledge of deleted pulls, potentially redownloading them
    WipeDeleted,
}

#[derive(Subcommand, Debug)]
enum ProfileMode {
    List,
    Set {
        /// Name of profile to edit/create
        name: String,
    },
    Delete {
        /// Name of profile to delete
        name: String,
    },
}

#[derive(Subcommand, Debug)]
enum SeriesMode {
    List,
    Get {
        /// Name of series to get
        name: String,
    },
    Set {
        /// Name of series to edit/create
        name: String,
    },
    Delete {
        /// Name of series to delete
        name: String,
    },
}

#[derive(Subcommand, Debug)]
enum PullMode {
    List {
        #[clap(short, long)]
        profile: Option<String>,
        #[clap(short, long)]
        title_contains: Option<String>,
        #[clap(long)]
        title_is: Option<String>,
        #[clap(short, long)]
        season_is: Option<u32>,
        #[clap(short, long)]
        episode_is: Option<String>,
        #[clap(short = 'm', long)]
        state: Option<String>,
    },
    Delete {
        /// ID of pull to delete
        id: String,
    },
}

lazy_static::lazy_static! {
    static ref ARGS: Args = Args::parse();
    static ref API: Url = {
        if let Some(api) = &ARGS.api {
            api.clone()
        } else {
            let raw_api = std::env::var("NYAUSER_API").unwrap_or_default();
            if raw_api.is_empty() {
                "http://localhost:8000/".parse().unwrap()
            } else {
                raw_api.parse().expect("failed to parse NYAUSER_API as URL")
            }
        }
    };
    static ref API_USER: String = {
        if let Some(user) = &ARGS.user {
            user.clone()
        } else {
            let raw_user = std::env::var("NYAUSER_USER").unwrap_or_default();
            if raw_user.is_empty() {
                "nyauser".parse().unwrap()
            } else {
                raw_user
            }
        }
    };
    static ref API_PASS: String = {
        if let Some(pass) = &ARGS.pass {
            pass.clone()
        } else {
            let raw_pass = std::env::var("NYAUSER_PASS").unwrap_or_default();
            if raw_pass.is_empty() {
                "nyauser".parse().unwrap()
            } else {
                raw_pass
            }
        }
    };
    static ref CLIENT: Client = Client::new();
}

#[tokio::main]
async fn main() {
    lazy_static::initialize(&API);
    env_logger::Builder::new()
        .parse_env(
            env_logger::Env::default().default_filter_or(if ARGS.verbose {
                "debug"
            } else {
                "info"
            }),
        )
        .init();

    if let Err(e) = execute_mode(&ARGS.mode).await {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn api<'a>(method: Method, segments: impl IntoIterator<Item = &'a str>) -> Result<RequestBuilder> {
    let mut url = API.clone();
    for segment in segments {
        if segment.is_empty() {
            continue;
        }
        url.path_segments_mut()
            .unwrap()
            .pop_if_empty()
            .push(segment);
    }
    Ok(CLIENT
        .request(method, url)
        .basic_auth(&*API_USER, Some(&*API_PASS)))
}

#[derive(Table)]
struct ProfileTable {
    #[table(title = "Name")]
    name: String,
    #[table(title = "Search Prefix")]
    search_prefix: String,
    #[table(title = "Regex")]
    parse_regex: String,
    #[table(title = "Relocate Dir")]
    relocate: String,
}

impl From<Profile> for ProfileTable {
    fn from(value: Profile) -> Self {
        Self {
            name: value.name,
            search_prefix: value.search_prefix.unwrap_or_default(),
            parse_regex: value.parse_regex.to_string(),
            relocate: value.relocate.unwrap_or_default(),
        }
    }
}

#[derive(Table)]
struct SeriesTable {
    #[table(title = "Name")]
    name: String,
    #[table(title = "Profile")]
    profile: String,
    #[table(title = "Max Days Old")]
    max_days_old: String,
    #[table(title = "Relocate Dir")]
    relocate: String,
    #[table(title = "Relocate Season")]
    relocate_season: bool,
}

impl From<Series> for SeriesTable {
    fn from(value: Series) -> Self {
        Self {
            name: value.name,
            profile: value.profile,
            max_days_old: value
                .max_days_old
                .map(|x| x.to_string())
                .unwrap_or_default(),
            relocate: value.relocate.unwrap_or_default(),
            relocate_season: value.relocate_season,
        }
    }
}

#[derive(Table)]
struct PullTable {
    #[table(title = "Id")]
    id: String,
    #[table(title = "State")]
    state: PullState,
    #[table(title = "Title")]
    title: String,
    #[table(title = "Season")]
    season: u32,
    #[table(title = "Episode")]
    episode: Episode,
    #[table(title = "Profile")]
    profile: String,
    #[table(title = "Torrent ID")]
    torrent_id: String,
    #[table(title = "Torrent Hash")]
    torrent_hash: String,
}

impl From<PullEntryNamed> for PullTable {
    fn from(value: PullEntryNamed) -> Self {
        Self {
            id: value.id,
            state: value.pull_entry.state,
            title: value.pull_entry.result.parsed.title,
            season: value.pull_entry.result.parsed.season,
            episode: value.pull_entry.result.parsed.episode,
            profile: value.pull_entry.result.profile,
            torrent_id: value
                .pull_entry
                .torrent_id
                .map(|x| x.to_string())
                .unwrap_or_default(),
            torrent_hash: value.pull_entry.torrent_hash,
        }
    }
}

lazy_static::lazy_static! {
    static ref DEFAULT_PROFILE: Profile = Profile {
        name: String::new(),
        search_prefix: Some("subsplease 1080p".to_string()),
        parse_regex: RegexWrapper(Regex::new(r"\[SubsPlease\] (?P<title>.*?) (?:S(?P<season>[0-9]{1,2}) )?- (?P<episode>(?:SP)?[0-9]{1,3}(?:\.\d)?)(?:v[0-9])? \(1080p\) \[(?P<checksum>[0-9a-zA-Z]{8})\]\.mkv").unwrap()),
        relocate: None,
    };
    static ref DEFAULT_SERIES: Series = Series {
        name: String::new(),
        profile: "subsplease".to_string(),
        max_days_old: None,
        relocate: None,
        relocate_season: true,
    };
}

async fn execute_mode(mode: &Mode) -> Result<()> {
    match mode {
        Mode::Profile { mode } => match mode {
            ProfileMode::List => {
                let response = api(Method::GET, "/api/v1/profile".split('/'))?
                    .send()
                    .await?;
                if !response.status().is_success() {
                    bail!(
                        "Got HTTP Status: {}\n{}",
                        response.status(),
                        response.text().await?
                    );
                }
                let list: Vec<Profile> = response.json().await?;
                let list: Vec<ProfileTable> = list.into_iter().map(Into::into).collect();
                print_stdout(list.with_title()).unwrap();
            }
            ProfileMode::Set { name } => {
                let response = api(
                    Method::GET,
                    "/api/v1/profile".split('/').chain(once(&**name)),
                )?
                .send()
                .await?;
                let current: Profile = if response.status().is_success() {
                    response.json().await?
                } else if response.status() == StatusCode::NOT_FOUND {
                    let mut profile = DEFAULT_PROFILE.clone();
                    profile.name = name.clone();
                    profile
                } else {
                    bail!(
                        "Got HTTP Status on get: {}\n{}",
                        response.status(),
                        response.text().await?
                    );
                };
                let output =
                    scrawl::with(&serde_yaml::to_string(&current)?).map_err(|e| anyhow!("{e}"))?;
                let output = output.to_string().map_err(|e| anyhow!("{e}"))?;
                let mut new: Profile = serde_yaml::from_str(&output)?;
                new.name = name.clone();
                let response = api(
                    Method::POST,
                    "/api/v1/profile".split('/').chain(once(&**name)),
                )?
                .json(&new)
                .send()
                .await?;
                if !response.status().is_success() {
                    bail!(
                        "Got HTTP Status on update: {}\n{}",
                        response.status(),
                        response.text().await?
                    );
                }
            }
            ProfileMode::Delete { name } => {
                let response = api(
                    Method::DELETE,
                    "/api/v1/profile".split('/').chain(once(&**name)),
                )?
                .send()
                .await?;
                if !response.status().is_success() {
                    bail!(
                        "Got HTTP Status: {}\n{}",
                        response.status(),
                        response.text().await?
                    );
                }
            }
        },
        Mode::Pull { mode } => match mode {
            PullMode::List {
                profile,
                title_contains,
                title_is,
                season_is,
                episode_is,
                state,
            } => {
                let response = api(Method::GET, "/api/v1/pull".split('/'))?
                    .query(&PullEntryFilter {
                        profile: profile.clone(),
                        title_contains: title_contains.clone(),
                        title_is: title_is.clone(),
                        season_is: season_is.clone(),
                        episode_is: episode_is.as_ref().map(|x| x.parse().unwrap()),
                        state: state
                            .as_ref()
                            .map(|x| x.parse())
                            .transpose()
                            .map_err(|_| anyhow!("invalid state"))?,
                    })
                    .send()
                    .await?;
                if !response.status().is_success() {
                    bail!(
                        "Got HTTP Status: {}\n{}",
                        response.status(),
                        response.text().await?
                    );
                }
                let list: Vec<PullEntryNamed> = response.json().await?;
                let list: Vec<PullTable> = list.into_iter().map(Into::into).collect();
                print_stdout(list.with_title()).unwrap();
            }
            PullMode::Delete { id } => {
                let response = api(Method::DELETE, "/api/v1/pull".split('/').chain(once(&**id)))?
                    .send()
                    .await?;
                if !response.status().is_success() {
                    bail!(
                        "Got HTTP Status: {}\n{}",
                        response.status(),
                        response.text().await?
                    );
                }
            }
        },
        Mode::Series { mode } => match mode {
            SeriesMode::List => {
                let response = api(Method::GET, "/api/v1/series".split('/'))?
                    .send()
                    .await?;
                if !response.status().is_success() {
                    bail!(
                        "Got HTTP Status: {}\n{}",
                        response.status(),
                        response.text().await?
                    );
                }
                let list: Vec<Series> = response.json().await?;
                let list: Vec<SeriesTable> = list.into_iter().map(Into::into).collect();
                print_stdout(list.with_title()).unwrap();
            }
            SeriesMode::Get { name } => {
                let response = api(
                    Method::GET,
                    "/api/v1/series"
                        .split('/')
                        .chain(once(&**name))
                        .chain(once("status")),
                )?
                .send()
                .await?;
                if !response.status().is_success() {
                    bail!(
                        "Got HTTP Status: {}\n{}",
                        response.status(),
                        response.text().await?
                    );
                }
                let status: SeriesStatus = response.json().await?;
                println!("{}", serde_yaml::to_string(&status)?);
            }
            SeriesMode::Set { name } => {
                let response = api(
                    Method::GET,
                    "/api/v1/series".split('/').chain(once(&**name)),
                )?
                .send()
                .await?;
                let current: Series = if response.status().is_success() {
                    response.json().await?
                } else if response.status() == StatusCode::NOT_FOUND {
                    let mut series = DEFAULT_SERIES.clone();
                    series.name = name.clone();
                    series
                } else {
                    bail!(
                        "Got HTTP Status on get: {}\n{}",
                        response.status(),
                        response.text().await?
                    );
                };
                let output =
                    scrawl::with(&serde_yaml::to_string(&current)?).map_err(|e| anyhow!("{e}"))?;
                let output = output.to_string().map_err(|e| anyhow!("{e}"))?;
                let mut new: Series = serde_yaml::from_str(&output)?;
                new.name = name.clone();
                let response = api(
                    Method::POST,
                    "/api/v1/series".split('/').chain(once(&**name)),
                )?
                .json(&new)
                .send()
                .await?;
                if !response.status().is_success() {
                    bail!(
                        "Got HTTP Status on update: {}\n{}",
                        response.status(),
                        response.text().await?
                    );
                }
            }
            SeriesMode::Delete { name } => {
                let response = api(
                    Method::DELETE,
                    "/api/v1/series".split('/').chain(once(&**name)),
                )?
                .send()
                .await?;
                if !response.status().is_success() {
                    bail!(
                        "Got HTTP Status: {}\n{}",
                        response.status(),
                        response.text().await?
                    );
                }
            }
        },
        Mode::Scan => {
            let response = api(Method::GET, "/api/v1/proc/scan".split('/'))?
                .send()
                .await?;
            if !response.status().is_success() {
                bail!(
                    "Got HTTP Status: {}\n{}",
                    response.status(),
                    response.text().await?
                );
            }
        }
        Mode::Search => {
            let response = api(Method::GET, "/api/v1/proc/search".split('/'))?
                .send()
                .await?;
            if !response.status().is_success() {
                bail!(
                    "Got HTTP Status: {}\n{}",
                    response.status(),
                    response.text().await?
                );
            }
        }
        Mode::WipeDeleted => {
            let response = api(Method::GET, "/api/v1/proc/wipe_deleted".split('/'))?
                .send()
                .await?;
            if !response.status().is_success() {
                bail!(
                    "Got HTTP Status: {}\n{}",
                    response.status(),
                    response.text().await?
                );
            }
        }
    }
    Ok(())
}
