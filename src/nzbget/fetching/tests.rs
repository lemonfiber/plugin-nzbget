use lemonfiber_ports::service::{Fetching, Pulling};

use super::super::tests::{answer, client, methods};

fn status(paused: bool) -> String {
    answer(&format!(
        r#"{{"DownloadPaused":{paused},"DownloadRate":0}}"#
    ))
}

#[tokio::test]
async fn a_paused_download_is_stopped_in_both_the_senses_that_matter() {
    assert_eq!(
        client(vec![(200, &status(true))]).0.pulling().await.ok(),
        Some(Pulling::Stopped)
    );
    assert_eq!(
        client(vec![(200, &status(false))]).0.pulling().await.ok(),
        Some(Pulling::Fetching)
    );
}

#[tokio::test]
async fn stopping_it_is_confirmed_by_asking_its_state_rather_than_by_the_answer() {
    let (nzbget, http) = client(vec![(200, &answer("true")), (200, &status(true))]);
    assert_eq!(nzbget.stop().await.ok(), Some(Pulling::Stopped));
    assert_eq!(methods(&http), ["pausedownload", "status"]);
}

#[tokio::test]
async fn a_client_that_took_the_request_and_went_on_fetching_says_so() {
    let (nzbget, _) = client(vec![(200, &answer("true")), (200, &status(false))]);
    assert_eq!(nzbget.stop().await.ok(), Some(Pulling::Fetching));
}

#[tokio::test]
async fn starting_it_again_is_confirmed_the_same_way() {
    let (nzbget, http) = client(vec![(200, &answer("true")), (200, &status(false))]);
    assert_eq!(nzbget.resume().await.ok(), Some(Pulling::Fetching));
    assert_eq!(methods(&http), ["resumedownload", "status"]);
}

#[tokio::test]
async fn a_request_the_client_answers_that_it_would_not_carry_out_is_a_failure() {
    assert!(client(vec![(200, &answer("false"))])
        .0
        .stop()
        .await
        .is_err());
    assert!(client(vec![(200, &answer("false"))])
        .0
        .resume()
        .await
        .is_err());
}

#[tokio::test]
async fn a_client_that_will_not_answer_at_all_is_a_failure() {
    assert!(client(vec![(200, "not json")]).0.pulling().await.is_err());
    assert!(client(vec![(503, "")]).0.stop().await.is_err());
}
