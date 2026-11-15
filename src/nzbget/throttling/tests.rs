use lemonfiber_ports::service::{Failure, Hours, Rates, Throttled, Throttling, Wanted, Window};
use serde_json::json;

use super::super::tests::{answer, calls, client, methods, sent};

const MEBIBYTE: u64 = 1_048_576;

const NO_TASKS: &str = r#"[{"Name":"DownloadRate","Value":"0"}]"#;

const TWO_FIGURES: &str = r#"[
    {"Name":"Task1.Time","Value":"07:00"},{"Name":"Task1.WeekDays","Value":"1-7"},
    {"Name":"Task1.Command","Value":"downloadrate"},{"Name":"Task1.Param","Value":"1024"},
    {"Name":"Task2.Time","Value":"23:00"},{"Name":"Task2.WeekDays","Value":"1-7"},
    {"Name":"Task2.Command","Value":"downloadrate"},{"Name":"Task2.Param","Value":"0"}]"#;

fn status(limit: u64, rate: u64) -> String {
    answer(&format!(
        r#"{{"DownloadLimit":{limit},"DownloadRate":{rate},"DownloadPaused":false}}"#
    ))
}

fn around_the_clock(down: Option<u64>) -> Wanted {
    Wanted {
        active: Rates { down, up: Some(1) },
        quiet: Rates {
            down: None,
            up: None,
        },
        window: None,
    }
}

fn window(from_hour: u8, to_hour: u8) -> Window {
    Window {
        from_hour,
        from_minute: 0,
        to_hour,
        to_minute: 0,
    }
}

#[tokio::test]
async fn the_limit_is_what_nzbget_reports_with_no_upload_and_no_schedule() {
    let (nzbget, _) = client(vec![(200, &status(0, 0)), (200, &answer(NO_TASKS))]);
    assert_eq!(
        nzbget.throttled().await.ok(),
        Some(Throttled {
            rates: Rates {
                down: None,
                up: None
            },
            uploads: false,
            hours: None,
        })
    );
    let (scheduled, _) = client(vec![
        (200, &status(MEBIBYTE, 0)),
        (200, &answer(TWO_FIGURES)),
    ]);
    assert_eq!(
        scheduled.throttled().await.ok().and_then(|held| held.hours),
        Some(Hours::Active)
    );
}

#[tokio::test]
async fn around_the_clock_the_rate_is_set_kept_in_the_settings_and_read_back() {
    let (nzbget, http) = client(vec![
        (200, &answer(NO_TASKS)),
        (200, &answer("true")),
        (200, &answer("true")),
        (200, &status(MEBIBYTE, 0)),
        (200, &answer(NO_TASKS)),
    ]);
    let held = nzbget.restrain(&around_the_clock(Some(MEBIBYTE))).await;
    assert_eq!(held.ok().map(|held| held.rates.down), Some(Some(MEBIBYTE)));
    let asked = calls(&http);
    assert_eq!(
        methods(&http),
        ["loadconfig", "saveconfig", "rate", "status", "config"]
    );
    assert_eq!(
        asked.get(1).map(|(_, params)| params.clone()),
        Some(json!([[{"Name":"DownloadRate","Value":"1024"}]]))
    );
    assert_eq!(
        asked.get(2).map(|(_, params)| params.clone()),
        Some(json!([1024]))
    );
}

#[tokio::test]
async fn a_rate_the_settings_already_hold_is_only_set() {
    let (nzbget, http) = client(vec![
        (200, &answer(NO_TASKS)),
        (200, &answer("true")),
        (200, &status(0, 0)),
        (200, &answer(NO_TASKS)),
    ]);
    assert!(nzbget.restrain(&around_the_clock(None)).await.is_ok());
    assert_eq!(methods(&http), ["loadconfig", "rate", "status", "config"]);
}

#[tokio::test]
async fn the_household_s_hours_are_two_scheduled_turns_taken_up_by_a_reload() {
    let wanted = Wanted {
        window: Some(window(7, 23)),
        ..around_the_clock(Some(MEBIBYTE))
    };
    let (nzbget, http) = client(vec![
        (200, &answer(NO_TASKS)),
        (200, &status(0, 0)),
        (200, &answer("true")),
        (200, &answer("true")),
        (200, &answer(TWO_FIGURES)),
        (200, &status(MEBIBYTE, 0)),
        (200, &answer(TWO_FIGURES)),
    ]);
    let held = nzbget.restrain(&wanted).await;
    assert_eq!(held.ok().and_then(|held| held.hours), Some(Hours::Active));
    assert_eq!(
        methods(&http),
        [
            "loadconfig",
            "status",
            "saveconfig",
            "reload",
            "config",
            "status",
            "config"
        ]
    );
    assert!(sent(&http, r#"{"Name":"Task1.Time","Value":"07:00"}"#));
    assert!(sent(&http, r#"{"Name":"Task2.Param","Value":"0"}"#));
}

#[tokio::test]
async fn hours_that_are_the_same_both_sides_are_held_around_the_clock() {
    let wanted = Wanted {
        window: Some(window(7, 23)),
        quiet: Rates {
            down: Some(MEBIBYTE),
            up: None,
        },
        ..around_the_clock(Some(MEBIBYTE))
    };
    let (nzbget, http) = client(vec![
        (200, &answer(TWO_FIGURES)),
        (200, &status(0, 0)),
        (200, &answer("true")),
        (200, &answer("true")),
        (200, &answer(NO_TASKS)),
        (200, &status(MEBIBYTE, 0)),
        (200, &answer(NO_TASKS)),
    ]);
    assert!(nzbget.restrain(&wanted).await.is_ok());
    assert_eq!(
        methods(&http),
        [
            "loadconfig",
            "status",
            "saveconfig",
            "reload",
            "config",
            "status",
            "config"
        ]
    );
    assert!(sent(&http, r#"{"Name":"DownloadRate","Value":"1024"}"#));
    assert!(!sent(&http, "Task1"));
}

#[tokio::test]
async fn a_window_off_the_clock_is_refused_before_anything_is_asked() {
    for off in [
        window(24, 7),
        window(7, 24),
        Window {
            from_minute: 60,
            ..window(7, 23)
        },
        Window {
            to_minute: 60,
            ..window(7, 23)
        },
    ] {
        let (nzbget, http) = client(Vec::new());
        let refused = nzbget
            .restrain(&Wanted {
                window: Some(off),
                ..around_the_clock(Some(MEBIBYTE))
            })
            .await;
        assert!(
            matches!(&refused, Err(Failure::Refused { detail, .. })
                if detail == "that window is not hours and minutes of a day"),
            "{refused:?}"
        );
        assert!(http.requests().is_empty());
    }
}

#[tokio::test]
async fn what_it_is_moving_is_what_nzbget_reports_and_nothing_up() {
    let (nzbget, _) = client(vec![(200, &status(0, 4096))]);
    assert_eq!(
        nzbget.moving().await.ok(),
        Some(Rates {
            down: Some(4096),
            up: None
        })
    );
    assert!(client(vec![(503, "")]).0.moving().await.is_err());
}
