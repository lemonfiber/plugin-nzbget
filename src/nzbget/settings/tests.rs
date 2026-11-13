use super::super::tests::{answer, calls, client, methods, sent, settings};
use super::{kilobytes, Entry, Settings};

fn written(pairs: &[(&str, &str)]) -> String {
    serde_json::to_string(&settings(pairs)).unwrap_or_default()
}

#[test]
fn a_setting_is_read_and_set_by_its_name_in_any_case() {
    let mut held = settings(&[("DestDir", "/config/downloads")]);
    assert_eq!(held.get("destdir"), "/config/downloads");
    assert_eq!(held.get("InterDir"), "");
    held.set("DESTDIR", "/data");
    held.set("InterDir", "/tmp");
    assert_eq!(
        held,
        settings(&[("DestDir", "/data"), ("InterDir", "/tmp")])
    );
}

#[test]
fn numbered_entries_are_gathered_by_number_whatever_order_they_are_held_in() {
    let held = settings(&[
        ("Category2.Name", "tv"),
        ("Category1.Name", "movies"),
        ("Category1.DestDir", "/films"),
        ("category10.name", "books"),
        ("Category.Name", "nothing"),
        ("CategoryX.Name", "nothing"),
        ("Categ", "nothing"),
        ("Category3", "nothing"),
        ("Server1.Name", "elsewhere"),
    ]);
    assert_eq!(
        held.numbered("Category"),
        [
            Entry::of(&[("Name", "movies"), ("DestDir", "/films")]),
            Entry::of(&[("Name", "tv")]),
            Entry::of(&[("name", "books")]),
        ]
    );
}

#[test]
fn numbered_entries_are_written_back_numbered_from_one_after_the_rest() {
    let mut held = settings(&[
        ("Category1.Name", "movies"),
        ("DestDir", "/data"),
        ("Category3.Name", "tv"),
    ]);
    let mut entries = held.numbered("Category");
    if let Some(first) = entries.first_mut() {
        first.set("DestDir", "/films");
        first.set("Unpack", "yes");
    }
    held.renumber("Category", &entries);
    assert_eq!(
        held,
        settings(&[
            ("DestDir", "/data"),
            ("Category1.Name", "movies"),
            ("Category1.DestDir", "/films"),
            ("Category1.Unpack", "yes"),
            ("Category2.Name", "tv"),
        ])
    );
    assert_eq!(Entry::of(&[("Name", "tv")]).get("missing"), "");
}

#[test]
fn a_rate_is_whole_kilobytes_rounded_up_and_no_rate_is_no_limit() {
    assert_eq!(kilobytes(None), 0);
    assert_eq!(kilobytes(Some(1)), 1);
    assert_eq!(kilobytes(Some(1024)), 1);
    assert_eq!(kilobytes(Some(1_048_577)), 1025);
}

const RUNNING: &str = r#"[{"Name":"Category1.Name","Value":"tv"}]"#;

#[tokio::test]
async fn reloading_writes_the_settings_waits_for_them_and_stops_a_stopped_client_again() {
    let (nzbget, http) = client(vec![
        (
            200,
            &answer(r#"{"DownloadPaused":true,"DownloadLimit":2048}"#),
        ),
        (200, &answer("true")),
        (200, &answer("true")),
        (503, ""),
        (200, &answer("[]")),
        (200, &answer(RUNNING)),
        (200, &answer("true")),
    ]);
    let running = nzbget
        .reloaded(settings(&[("DestDir", "/data")]), true, |running| {
            running.get("Category1.Name") == "tv"
        })
        .await;
    assert_eq!(running.ok(), Some(settings(&[("Category1.Name", "tv")])));
    assert_eq!(
        methods(&http),
        [
            "status",
            "saveconfig",
            "reload",
            "config",
            "config",
            "config",
            "pausedownload"
        ]
    );
    let saved = calls(&http)
        .into_iter()
        .find(|(method, _)| method == "saveconfig")
        .map(|(_, params)| params.to_string());
    assert_eq!(
        saved,
        Some(format!(
            "[{}]",
            written(&[("DestDir", "/data"), ("DownloadRate", "2")])
        ))
    );
}

#[tokio::test]
async fn reloading_without_carrying_leaves_the_rate_and_a_running_client_running() {
    let (nzbget, http) = client(vec![
        (
            200,
            &answer(r#"{"DownloadPaused":false,"DownloadLimit":2048}"#),
        ),
        (200, &answer("true")),
        (200, &answer("true")),
        (200, &answer(RUNNING)),
    ]);
    assert!(nzbget
        .reloaded(settings(&[("DestDir", "/data")]), false, |_| true)
        .await
        .is_ok());
    assert_eq!(methods(&http), ["status", "saveconfig", "reload", "config"]);
    assert!(!sent(&http, "DownloadRate"));
}

#[tokio::test]
async fn a_client_that_never_takes_the_settings_up_is_a_failure_that_says_so() {
    let mut replies = vec![
        (200, answer(r#"{"DownloadPaused":false,"DownloadLimit":0}"#)),
        (200, answer("true")),
        (200, answer("true")),
    ];
    replies.extend((0..super::RELOAD_TRIES).map(|_| (200, answer("[]"))));
    let (nzbget, _) = client(
        replies
            .iter()
            .map(|(status, body)| (*status, body.as_str()))
            .collect(),
    );
    let refused = nzbget
        .reloaded(Settings::default(), false, |running| {
            running.get("x") == "y"
        })
        .await;
    assert!(
        matches!(&refused, Err(lemonfiber_ports::service::Failure::Refused { detail, .. })
            if detail == "the settings were written and the client is running with others"),
        "{refused:?}"
    );
}

#[tokio::test]
async fn a_client_that_stops_answering_through_a_reload_is_unavailable() {
    let mut replies = vec![
        (200, answer(r#"{"DownloadPaused":false,"DownloadLimit":0}"#)),
        (200, answer("true")),
        (200, answer("true")),
    ];
    replies.extend((0..super::RELOAD_TRIES).map(|_| (503, String::new())));
    let (nzbget, _) = client(
        replies
            .iter()
            .map(|(status, body)| (*status, body.as_str()))
            .collect(),
    );
    assert!(matches!(
        nzbget.reloaded(Settings::default(), false, |_| true).await,
        Err(lemonfiber_ports::service::Failure::Unavailable { .. })
    ));
}

#[test]
fn settings_never_print_what_they_hold() {
    let held = settings(&[("ControlPassword", "hunter2")]);
    let printed = format!("{held:?}");
    assert!(printed.contains("ControlPassword"));
    assert!(!printed.contains("hunter2"));
}

#[tokio::test]
async fn a_client_that_will_not_write_or_reload_is_a_failure() {
    let state = answer(r#"{"DownloadPaused":false,"DownloadLimit":0}"#);
    for replies in [
        vec![(200, state.as_str()), (200, &answer("false"))],
        vec![
            (200, state.as_str()),
            (200, &answer("true")),
            (200, &answer("false")),
        ],
    ] {
        let (nzbget, _) = client(replies);
        assert!(nzbget
            .reloaded(Settings::default(), false, |_| true)
            .await
            .is_err());
    }
}
