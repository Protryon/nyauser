use std::collections::HashMap;

use regex::{Match, Regex};
use serde::{Deserialize, Deserializer, Serialize};

fn deserialize_regex<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Regex, D::Error> {
    let string = String::deserialize(deserializer)?;
    Regex::new(&*string).map_err(|e| {
        serde::de::Error::invalid_value(
            serde::de::Unexpected::Str(&*string),
            &&*format!("invalid regex: {:?}", e),
        )
    })
}

#[derive(Deserialize)]
pub struct ProfileConfig {
    /// initial parts of search phrase, of which is followed by space and series name
    pub search_prefix: Option<String>,
    /// torrent name parsing regex
    #[serde(deserialize_with = "deserialize_regex")]
    pub parse_regex: Regex,
    /// if set, is a default path for series relocation. I.e. `relocate`/<series-name>/Season X/episode1.mp4
    pub relocate: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct StandardEpisode {
    pub title: String,
    pub season: u32,
    pub episode: u32,
    pub checksum: u32,
    pub ext: HashMap<String, String>,
}

impl ProfileConfig {
    pub fn parse_name<'s, 't>(&'s self, name: &'t str) -> Option<StandardEpisode> {
        let mut out = StandardEpisode::default();
        out.season = 1;
        let captures = self.parse_regex.captures(name)?;
        for name in self.parse_regex.capture_names().filter_map(|x| x) {
            let value = match captures.name(name).as_ref().map(Match::as_str) {
                Some(x) => x,
                None => continue,
            };
            match name {
                "title" => out.title = value.to_string(),
                "season" => out.season = value.parse().ok()?,
                "episode" => out.episode = value.parse().ok()?,
                "checksum" => {
                    out.checksum = u32::from_le_bytes(hex::decode(value).ok()?.try_into().ok()?)
                }
                name => {
                    out.ext.insert(name.to_string(), value.to_string());
                }
            }
        }
        Some(out)
    }
}
