use std::time::Duration;

use lemonfiber_ports::service::{Download, TransferState, Transfers};

use super::super::tests::{answer, client, methods};
use super::progress;

const DOWNLOADING: &str = r#"{"NZBID":7,"NZBName":"Show.S01E01","Status":"DOWNLOADING","Category":"tv",
    "FileSizeLo":3000,"FileSizeHi":0,"RemainingSizeLo":1500,"RemainingSizeHi":0,
    "PausedSizeLo":1000,"PausedSizeHi":0,
    "Parameters":[{"Name":"drone","Value":"9f2c"},{"Name":"*Unpack:","Value":"yes"}]}"#;

const QUEUED: &str = r#"{"NZBID":8,"NZBName":"Film.2024","Status":"QUEUED","Category":"",
    "FileSizeLo":1000,"FileSizeHi":0,"RemainingSizeLo":1000,"RemainingSizeHi":0,
    "Parameters":[{"Name":"*Unpack:","Value":"yes"}]}"#;

const UNPACKING: &str = r#"{"NZBID":9,"NZBName":"Album","Status":"UNPACKING","Category":"music",
    "FileSizeLo":2000,"FileSizeHi":0,"RemainingSizeLo":0,"RemainingSizeHi":0}"#;

fn status(rate: u64, paused: bool) -> String {
    answer(&format!(
        r#"{{"DownloadRate":{rate},"DownloadPaused":{paused}}}"#
    ))
}

#[tokio::test]
async fn each_group_names_its_id_category_state_and_what_is_left_of_it() {
    let groups = answer(&format!("[{DOWNLOADING},{QUEUED},{UNPACKING}]"));
    let (nzbget, http) = client(vec![(200, &groups), (200, &status(250, false))]);
    assert_eq!(
        nzbget.transfers().await.ok(),
        Some(vec![
            Download {
                name: "Show.S01E01".to_owned(),
                progress: 75,
                speed: Some(250),
                eta: Some(Duration::from_secs(2)),
                remaining: Some(500),
                id: Some("9f2c".to_owned()),
                category: Some("tv".to_owned()),
                state: Some(TransferState::Downloading),
                added: None,
            },
            Download {
                name: "Film.2024".to_owned(),
                progress: 0,
                speed: Some(0),
                eta: None,
                remaining: Some(1000),
                id: Some("8".to_owned()),
                category: None,
                state: Some(TransferState::Queued),
                added: None,
            },
            Download {
                name: "Album".to_owned(),
                progress: 100,
                speed: Some(0),
                eta: None,
                remaining: Some(0),
                id: Some("9".to_owned()),
                category: Some("music".to_owned()),
                state: None,
                added: None,
            },
        ])
    );
    assert_eq!(methods(&http), ["listgroups", "status"]);
}

#[tokio::test]
async fn a_paused_client_says_every_waiting_group_is_paused_and_moving_nothing() {
    let groups = answer(&format!("[{DOWNLOADING},{QUEUED},{UNPACKING}]"));
    let (nzbget, _) = client(vec![(200, &groups), (200, &status(0, true))]);
    let states: Vec<_> = nzbget
        .transfers()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|download| (download.state, download.eta))
        .collect();
    assert_eq!(
        states,
        [
            (Some(TransferState::Paused), None),
            (Some(TransferState::Paused), None),
            (None, None)
        ]
    );
}

#[tokio::test]
async fn a_group_paused_on_its_own_is_paused_and_an_empty_queue_is_nothing() {
    let paused = answer(
        r#"[{"NZBID":3,"NZBName":"a","Status":"PAUSED","FileSizeLo":10,"RemainingSizeLo":10}]"#,
    );
    let (nzbget, _) = client(vec![(200, &paused), (200, &status(0, false))]);
    assert_eq!(
        nzbget
            .transfers()
            .await
            .ok()
            .and_then(|downloads| downloads.first().and_then(|one| one.state)),
        Some(TransferState::Paused)
    );
    let (empty, _) = client(vec![(200, &answer("[]")), (200, &status(0, false))]);
    assert_eq!(empty.transfers().await.ok(), Some(Vec::new()));
}

#[tokio::test]
async fn a_queue_or_a_rate_that_cannot_be_read_is_a_failure() {
    assert!(client(vec![(200, "not json")]).0.transfers().await.is_err());
    assert!(client(vec![(200, &answer("[]")), (503, "")])
        .0
        .transfers()
        .await
        .is_err());
}

#[test]
fn progress_is_what_is_done_of_the_whole_and_nothing_of_nothing() {
    assert_eq!(progress(0, 0), 0);
    assert_eq!(progress(200, 50), 75);
    assert_eq!(progress(200, 300), 0);
}
