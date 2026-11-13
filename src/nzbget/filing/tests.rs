use lemonfiber_ports::service::{Failure, Filed, Filing};

use super::super::tests::{answer, calls, client, methods, sent};

const RUNNING: &str = r#"[
    {"Name":"DestDir","Value":"/config/downloads/complete/"},
    {"Name":"AppendCategoryDir","Value":"yes"},
    {"Name":"Category1.Name","Value":"movies"},{"Name":"Category1.DestDir","Value":""},
    {"Name":"Category2.Name","Value":"tv"},{"Name":"Category2.DestDir","Value":"/data/tv"},
    {"Name":"Category3.Name","Value":" "}]"#;

const SAVED: &str = r#"[
    {"Name":"DestDir","Value":"${MainDir}/downloads/complete"},
    {"Name":"Category1.Name","Value":"movies"},{"Name":"Category1.DestDir","Value":""},
    {"Name":"Category2.Name","Value":"tv"},{"Name":"Category2.DestDir","Value":"/data/tv"}]"#;

const TAKEN_UP: &str = r#"[
    {"Name":"DestDir","Value":"/config/downloads/complete"},
    {"Name":"AppendCategoryDir","Value":"yes"},
    {"Name":"Category1.Name","Value":"movies"},{"Name":"Category1.DestDir","Value":"/data/films"},
    {"Name":"Category2.Name","Value":"tv"},{"Name":"Category2.DestDir","Value":"/data/tv"},
    {"Name":"Category3.Name","Value":"books"},{"Name":"Category3.DestDir","Value":"/data/books"}]"#;

fn filed(category: &str, folder: &str) -> Filed {
    Filed {
        category: category.to_owned(),
        folder: folder.to_owned(),
    }
}

#[tokio::test]
async fn each_category_files_under_its_own_folder_or_its_name_below_the_completed_one() {
    let (nzbget, http) = client(vec![(200, &answer(RUNNING))]);
    assert_eq!(
        nzbget.filed().await.ok(),
        Some(vec![
            filed("movies", "/config/downloads/complete/movies"),
            filed("tv", "/data/tv"),
        ])
    );
    assert_eq!(methods(&http), ["config"]);
}

#[tokio::test]
async fn without_appending_every_category_lands_in_the_completed_folder_itself() {
    let alone = answer(
        r#"[{"Name":"DestDir","Value":"/done"},{"Name":"AppendCategoryDir","Value":"no"},
            {"Name":"Category1.Name","Value":"movies"}]"#,
    );
    let (nzbget, _) = client(vec![(200, &alone)]);
    assert_eq!(
        nzbget.filed().await.ok(),
        Some(vec![filed("movies", "/done")])
    );
}

#[tokio::test]
async fn filing_sets_a_held_category_s_folder_makes_a_new_one_and_reads_them_back() {
    let (nzbget, http) = client(vec![
        (200, &answer(RUNNING)),
        (200, &answer(SAVED)),
        (
            200,
            &answer(r#"{"DownloadPaused":false,"DownloadLimit":0}"#),
        ),
        (200, &answer("true")),
        (200, &answer("true")),
        (200, &answer(TAKEN_UP)),
    ]);
    let filing = nzbget
        .file_under(&[
            filed("Movies", "/data/films"),
            filed("books", "/data/books"),
        ])
        .await;
    assert!(filing.is_ok(), "{filing:?}");
    assert_eq!(
        methods(&http),
        [
            "config",
            "loadconfig",
            "status",
            "saveconfig",
            "reload",
            "config"
        ]
    );
    let saved = calls(&http)
        .into_iter()
        .find(|(method, _)| method == "saveconfig")
        .map(|(_, params)| params);
    assert_eq!(
        saved,
        Some(serde_json::json!([[
            {"Name":"DestDir","Value":"${MainDir}/downloads/complete"},
            {"Name":"Category1.Name","Value":"movies"},{"Name":"Category1.DestDir","Value":"/data/films"},
            {"Name":"Category2.Name","Value":"tv"},{"Name":"Category2.DestDir","Value":"/data/tv"},
            {"Name":"Category3.Name","Value":"books"},{"Name":"Category3.DestDir","Value":"/data/books"},
            {"Name":"DownloadRate","Value":"0"}
        ]]))
    );
}

#[tokio::test]
async fn folders_nzbget_already_files_under_are_left_alone() {
    let (nzbget, http) = client(vec![(200, &answer(RUNNING))]);
    assert!(nzbget
        .file_under(&[
            filed("TV", "/data/tv"),
            filed("movies", "/config/downloads/complete/movies")
        ])
        .await
        .is_ok());
    assert_eq!(methods(&http), ["config"]);
}

#[tokio::test]
async fn a_category_or_folder_nzbget_could_not_hold_is_refused_before_anything_is_asked() {
    for misshapen in [
        filed("", "/data/x"),
        filed("a/b", "/data/x"),
        filed("tv", "relative"),
        filed("tv", "/data/${MainDir}"),
        filed("tv", "/data/\nx"),
        filed("tv", "/data/../etc"),
    ] {
        let (nzbget, http) = client(Vec::new());
        let refused = nzbget.file_under(&[misshapen]).await;
        assert!(
            matches!(&refused, Err(Failure::Refused { detail, .. })
                if detail.ends_with("is not a category and folder NZBGet could hold")),
            "{refused:?}"
        );
        assert!(http.requests().is_empty());
    }
}

#[tokio::test]
async fn categories_that_cannot_be_read_are_a_failure() {
    assert!(client(vec![(503, "")]).0.filed().await.is_err());
    let (nzbget, http) = client(vec![(200, &answer(RUNNING)), (503, "")]);
    assert!(nzbget
        .file_under(&[filed("books", "/data/books")])
        .await
        .is_err());
    assert!(!sent(&http, "saveconfig"));
}
