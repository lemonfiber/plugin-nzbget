use std::collections::BTreeMap;

use lemonfiber_ports::service::Failure;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Nzbget, KILOBYTE, PAUSE, STATUS};

pub(super) const DOWNLOAD_RATE: &str = "DownloadRate";

const LIVE: &str = "config";

const SAVED: &str = "loadconfig";

const SAVE: &str = "saveconfig";

const RELOAD: &str = "reload";

const RELOAD_TRIES: u32 = 20;

const FIELD: char = '.';

/// One setting as NZBGet names and holds it.
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Setting {
    name: String,
    value: String,
}

/// Fields by name, each read and written by its name in any case.
#[derive(Clone, PartialEq, Eq, Default)]
pub(super) struct Entry(Vec<(String, String)>);

/// Every setting NZBGet holds, in the order it holds them.
#[derive(Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(from = "Vec<Setting>", into = "Vec<Setting>")]
pub(super) struct Settings(Entry);

impl Entry {
    pub(super) fn of(fields: &[(&str, &str)]) -> Self {
        Self(
            fields
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
        )
    }

    pub(super) fn get(&self, field: &str) -> &str {
        self.0
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(field))
            .map_or("", |(_, value)| value.as_str())
    }

    pub(super) fn set(&mut self, field: &str, value: &str) {
        match self
            .0
            .iter_mut()
            .find(|(name, _)| name.eq_ignore_ascii_case(field))
        {
            Some((_, held)) => value.clone_into(held),
            None => self.0.push((field.to_owned(), value.to_owned())),
        }
    }
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_list()
            .entries(self.0.iter().map(|(name, _)| name))
            .finish()
    }
}

impl std::fmt::Debug for Settings {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<Vec<Setting>> for Settings {
    fn from(held: Vec<Setting>) -> Self {
        Self(Entry(
            held.into_iter()
                .map(|setting| (setting.name, setting.value))
                .collect(),
        ))
    }
}

impl From<Settings> for Vec<Setting> {
    fn from(settings: Settings) -> Self {
        settings
            .0
             .0
            .into_iter()
            .map(|(name, value)| Setting { name, value })
            .collect()
    }
}

impl Settings {
    pub(super) fn get(&self, name: &str) -> &str {
        self.0.get(name)
    }

    pub(super) fn set(&mut self, name: &str, value: &str) {
        self.0.set(name, value);
    }

    /// Every entry numbered under `prefix`, in the order of their numbers.
    pub(super) fn numbered(&self, prefix: &str) -> Vec<Entry> {
        let mut found: BTreeMap<u32, Entry> = BTreeMap::new();
        for (name, value) in &self.0 .0 {
            if let Some((number, field)) = numbered_as(name, prefix) {
                found
                    .entry(number)
                    .or_default()
                    .0
                    .push((field.to_owned(), value.clone()));
            }
        }
        found.into_values().collect()
    }

    /// The entries numbered under `prefix` replaced by `entries`, numbered from one.
    pub(super) fn renumber(&mut self, prefix: &str, entries: &[Entry]) {
        self.0
             .0
            .retain(|(name, _)| numbered_as(name, prefix).is_none());
        for (number, entry) in (1_u32..).zip(entries) {
            for (field, value) in &entry.0 {
                self.0
                     .0
                    .push((format!("{prefix}{number}{FIELD}{field}"), value.clone()));
            }
        }
    }
}

fn numbered_as<'a>(name: &'a str, prefix: &str) -> Option<(u32, &'a str)> {
    let head = name.get(..prefix.len())?;
    if !head.eq_ignore_ascii_case(prefix) {
        return None;
    }
    let (number, field) = name.get(prefix.len()..)?.split_once(FIELD)?;
    if number.is_empty() || !number.bytes().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    Some((number.parse().ok()?, field))
}

/// A download rate in the kilobytes a second NZBGet's settings and `rate` take, where
/// nothing at all is no limit.
pub(super) fn kilobytes(limit: Option<u64>) -> u64 {
    limit.map_or(0, |bytes| bytes.div_ceil(KILOBYTE).max(1))
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Held {
    #[serde(default)]
    download_paused: bool,
    #[serde(default)]
    download_limit: u64,
}

impl Nzbget {
    /// The settings NZBGet is running with.
    pub(super) async fn live(&self, whenever: &str) -> Result<Settings, Failure> {
        self.call(LIVE, json!([]), whenever).await
    }

    /// The settings as NZBGet's settings file holds them, before any is resolved.
    pub(super) async fn saved(&self, whenever: &str) -> Result<Settings, Failure> {
        self.call(SAVED, json!([]), whenever).await
    }

    /// Write `next` as the whole of NZBGet's settings file.
    pub(super) async fn save(&self, next: &Settings) -> Result<(), Failure> {
        self.asked(SAVE, json!([next]), "the settings could not be written")
            .await
    }

    /// Write `next`, have NZBGet run with it, and answer with what it runs with once it
    /// holds what `took` looks for.
    ///
    /// NZBGet starts fetching again at its own rate on a reload, so a client stopped
    /// before it is stopped again after; where `carrying` the rate it is held to is
    /// written with the rest, so the reload keeps it.
    pub(super) async fn reloaded(
        &self,
        mut next: Settings,
        carrying: bool,
        took: impl Fn(&Settings) -> bool + Send + Sync,
    ) -> Result<Settings, Failure> {
        let before: Held = self
            .call(STATUS, json!([]), "the client's state could not be read")
            .await?;
        if carrying {
            next.set(
                DOWNLOAD_RATE,
                &(before.download_limit / KILOBYTE).to_string(),
            );
        }
        self.save(&next).await?;
        self.asked(RELOAD, json!([]), "the settings could not be taken up")
            .await?;
        let running = self.settled(&took).await?;
        if before.download_paused {
            self.asked(PAUSE, json!([]), "the client could not be stopped again")
                .await?;
        }
        Ok(running)
    }

    async fn settled(
        &self,
        took: &(impl Fn(&Settings) -> bool + Sync),
    ) -> Result<Settings, Failure> {
        let mut failed = None;
        for _ in 0..RELOAD_TRIES {
            tokio::time::sleep(self.settling).await;
            match self.live("the settings could not be read back").await {
                Ok(running) if took(&running) => return Ok(running),
                Ok(_) => failed = None,
                Err(failure) => failed = Some(failure),
            }
        }
        Err(failed.unwrap_or_else(|| {
            self.endpoint
                .refused("the settings were written and the client is running with others")
        }))
    }
}

#[cfg(test)]
mod tests;
