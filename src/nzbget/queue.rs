use std::time::Duration;

use async_trait::async_trait;
use lemonfiber_ports::service::{Download, Failure, TransferState, Transfers};
use serde::Deserialize;
use serde_json::json;

use super::{bytes, Nzbget, STATUS};

const MOST_PERCENT: u64 = 100;

const DOWNLOADING: &str = "DOWNLOADING";

const STATES: [(&str, TransferState); 4] = [
    (DOWNLOADING, TransferState::Downloading),
    ("FETCHING", TransferState::Downloading),
    ("QUEUED", TransferState::Queued),
    ("PAUSED", TransferState::Paused),
];

const CURATOR_ID: &str = "drone";

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Group {
    #[serde(rename = "NZBID")]
    nzb_id: u64,
    #[serde(rename = "NZBName")]
    nzb_name: String,
    status: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    file_size_lo: u64,
    #[serde(default)]
    file_size_hi: u64,
    #[serde(default)]
    remaining_size_lo: u64,
    #[serde(default)]
    remaining_size_hi: u64,
    #[serde(default)]
    paused_size_lo: u64,
    #[serde(default)]
    paused_size_hi: u64,
    #[serde(default)]
    parameters: Vec<Parameter>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Parameter {
    name: String,
    value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Moving {
    #[serde(default)]
    download_rate: u64,
    #[serde(default)]
    download_paused: bool,
}

#[async_trait]
impl Transfers for Nzbget {
    async fn transfers(&self) -> Result<Vec<Download>, Failure> {
        let groups: Vec<Group> = self
            .call("listgroups", json!([]), "the queue could not be read")
            .await?;
        let moving: Moving = self
            .call(STATUS, json!([]), "the transfer rate could not be read")
            .await?;
        Ok(groups
            .into_iter()
            .map(|group| download_of(group, &moving))
            .collect())
    }
}

fn download_of(group: Group, moving: &Moving) -> Download {
    let paused = bytes(group.paused_size_lo, group.paused_size_hi);
    let whole = bytes(group.file_size_lo, group.file_size_hi).saturating_sub(paused);
    let left = bytes(group.remaining_size_lo, group.remaining_size_hi).saturating_sub(paused);
    let downloading = group.status == DOWNLOADING;
    let speed = if downloading { moving.download_rate } else { 0 };
    let state = state_of(&group.status).map(|said| match said {
        TransferState::Downloading | TransferState::Queued if moving.download_paused => {
            TransferState::Paused
        }
        said => said,
    });
    let id = group
        .parameters
        .iter()
        .find(|parameter| parameter.name == CURATOR_ID && !parameter.value.is_empty())
        .map_or_else(
            || group.nzb_id.to_string(),
            |parameter| parameter.value.clone(),
        );
    Download {
        name: group.nzb_name,
        progress: progress(whole, left),
        speed: Some(speed),
        eta: (speed > 0)
            .then(|| left / speed)
            .filter(|seconds| *seconds > 0)
            .map(Duration::from_secs),
        remaining: Some(left),
        id: Some(id),
        category: Some(group.category).filter(|category| !category.is_empty()),
        state,
        added: None,
    }
}

fn progress(whole: u64, left: u64) -> u8 {
    if whole == 0 {
        return 0;
    }
    let done = whole.saturating_sub(left).saturating_mul(MOST_PERCENT) / whole;
    u8::try_from(done.min(MOST_PERCENT)).unwrap_or(u8::MAX)
}

fn state_of(status: &str) -> Option<TransferState> {
    STATES
        .iter()
        .find(|(named, _)| *named == status)
        .map(|(_, said)| *said)
}

#[cfg(test)]
mod tests;
