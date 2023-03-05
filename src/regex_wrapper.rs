use std::borrow::Cow;
use std::ops::Deref;

use regex::Regex;
use serde::{de::Unexpected, Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct RegexWrapper(pub Regex);

impl PartialEq for RegexWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Serialize for RegexWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.as_str().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RegexWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let r: Cow<'de, str> = Deserialize::deserialize(deserializer)?;

        Ok(Self(Regex::new(&r).map_err(|e| {
            serde::de::Error::invalid_value(Unexpected::Str(&r), &e.to_string().deref())
        })?))
    }
}

impl Deref for RegexWrapper {
    type Target = Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
