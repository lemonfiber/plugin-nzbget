use lemonfiber_ports::service::{Metering, Moved};

use super::super::tests::{answer, client, methods};

const VOLUMES: &str = r#"[
    {"ServerID":0,"FirstDay":20453,"TotalSizeLo":0,"TotalSizeHi":0,
     "BytesPerDays":[{"SizeLo":100,"SizeHi":0},{"SizeLo":0,"SizeHi":1},{"SizeLo":7,"SizeHi":0}]},
    {"ServerID":1,"FirstDay":20453,"BytesPerDays":[{"SizeLo":999,"SizeHi":0}]}]"#;

#[tokio::test]
async fn what_moved_in_a_month_is_every_account_s_days_in_it_with_nothing_up() {
    let (nzbget, http) = client(vec![(200, &answer(VOLUMES))]);
    assert_eq!(
        nzbget.moved("2025-12").await.ok(),
        Some(Moved {
            down: 100,
            up: 0,
            since_start: false,
        })
    );
    assert_eq!(methods(&http), ["servervolumes"]);
    let (later, _) = client(vec![(200, &answer(VOLUMES))]);
    assert_eq!(
        later.moved(" 2026-01 ").await.ok().map(|moved| moved.down),
        Some((1 << 32) + 7)
    );
}

#[tokio::test]
async fn a_month_with_no_days_or_a_client_with_no_total_moved_nothing() {
    let (nzbget, _) = client(vec![(200, &answer(VOLUMES))]);
    assert_eq!(
        nzbget.moved("2026-02").await.ok().map(|moved| moved.down),
        Some(0)
    );
    let (empty, _) = client(vec![(200, &answer("[]"))]);
    assert_eq!(
        empty.moved("2026-01").await.ok().map(|moved| moved.down),
        Some(0)
    );
    assert!(client(vec![(503, "")]).0.moved("2026-01").await.is_err());
}
