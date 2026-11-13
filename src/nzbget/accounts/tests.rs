use lemonfiber_manifest::Date;
use lemonfiber_ports::service::{UsenetAccount, UsenetAccounts};

use super::super::tests::{answer, client, methods};

const SERVERS: &str = r#"[
    {"Name":"Server1.Active","Value":"yes"},{"Name":"Server1.Name","Value":"Provider One"},
    {"Name":"Server1.Host","Value":"news.example"},
    {"Name":"Server2.Active","Value":"no"},{"Name":"Server2.Name","Value":" "},
    {"Name":"Server2.Host","Value":"backup.example"}]"#;

const VOLUMES: &str = r#"[
    {"ServerID":0,"FirstDay":20454,"TotalSizeLo":12,"TotalSizeHi":0,"BytesPerDays":[]},
    {"ServerID":1,"FirstDay":20454,"TotalSizeLo":12,"TotalSizeHi":1,
     "BytesPerDays":[{"SizeLo":5,"SizeHi":0},{"SizeLo":7,"SizeHi":0}]}]"#;

#[tokio::test]
async fn each_server_is_an_account_with_what_nzbget_measured_pulling_from_it() {
    let (nzbget, http) = client(vec![(200, &answer(SERVERS)), (200, &answer(VOLUMES))]);
    assert_eq!(
        nzbget.accounts().await.ok(),
        Some(vec![
            UsenetAccount {
                name: "Provider One".to_owned(),
                enabled: true,
                quota: None,
                downloaded: (1 << 32) + 12,
                daily: vec![
                    (
                        Date {
                            year: 2026,
                            month: 1,
                            day: 1
                        },
                        5
                    ),
                    (
                        Date {
                            year: 2026,
                            month: 1,
                            day: 2
                        },
                        7
                    ),
                ],
                expires_on: None,
                standing: None,
            },
            UsenetAccount {
                name: "backup.example".to_owned(),
                enabled: false,
                quota: None,
                downloaded: 0,
                daily: Vec::new(),
                expires_on: None,
                standing: None,
            },
        ])
    );
    assert_eq!(methods(&http), ["config", "servervolumes"]);
}

#[tokio::test]
async fn a_client_given_no_server_holds_no_account_and_one_unread_is_a_failure() {
    let (nzbget, _) = client(vec![(200, &answer("[]")), (200, &answer(VOLUMES))]);
    assert_eq!(nzbget.accounts().await.ok(), Some(Vec::new()));
    assert!(client(vec![(503, "")]).0.accounts().await.is_err());
    assert!(client(vec![(200, &answer(SERVERS)), (200, "not json")])
        .0
        .accounts()
        .await
        .is_err());
}
