use std::sync::Arc;
use std::time::Duration;

use lemonfiber_fixtures::http::{Answer, Fake};
use lemonfiber_ports::http::Method;
use lemonfiber_ports::service::{Failure, Fetching};
use serde_json::{json, Value};

use super::settings::Settings;
use super::{bytes, Nzbget};
use crate::credential::{Credential, Holding};

pub(super) const BASE: &str = "http://127.0.0.1:6789";

pub(super) fn account() -> Credential {
    Credential {
        username: "lemonfiber".to_owned(),
        password: ('d'..='s').collect(),
    }
}

pub(super) fn client(replies: Vec<(u16, &str)>) -> (Nzbget, Arc<Fake>) {
    let http = Fake::in_turn(
        replies
            .into_iter()
            .map(|(status, body)| Answer::reply(status, body.to_owned()))
            .collect(),
    );
    (
        Nzbget::new(http.clone(), BASE, Arc::new(account())).settling(Duration::ZERO),
        http,
    )
}

pub(super) fn answer(result: &str) -> String {
    format!(r#"{{"version":"1.1","id":0,"result":{result}}}"#)
}

pub(super) fn calls(http: &Fake) -> Vec<(String, Value)> {
    http.requests()
        .into_iter()
        .filter_map(|asked| {
            let body: Value = serde_json::from_str(asked.body.as_deref()?).ok()?;
            let method = body.get("method")?.as_str()?.to_owned();
            Some((method, body.get("params")?.clone()))
        })
        .collect()
}

pub(super) fn sent(http: &Fake, fragment: &str) -> bool {
    http.requests().iter().any(|asked| {
        asked
            .body
            .as_deref()
            .is_some_and(|body| body.contains(fragment))
    })
}

pub(super) fn methods(http: &Fake) -> Vec<String> {
    calls(http).into_iter().map(|(method, _)| method).collect()
}

pub(super) fn settings(pairs: &[(&str, &str)]) -> Settings {
    serde_json::from_value(json!(pairs
        .iter()
        .map(|(name, value)| json!({ "Name": name, "Value": value }))
        .collect::<Vec<_>>()))
    .unwrap_or_default()
}

struct Nothing;

impl Holding for Nothing {
    fn credential(&self) -> Result<Credential, String> {
        Err("upstream.json could not be read: not found".to_owned())
    }
}

#[tokio::test]
async fn every_call_is_posted_to_the_rpc_path_signed_in_with_the_account() {
    let (nzbget, http) = client(vec![(200, &answer(r#""26.3""#))]);
    assert!(nzbget.answers().await);
    let asked = http.requests();
    assert_eq!(asked.len(), 1);
    let Some(one) = asked.first() else {
        return;
    };
    assert_eq!(one.method, Method::Post);
    assert_eq!(one.url, format!("{BASE}/jsonrpc"));
    assert!(one.headers.contains(&(
        "Authorization".to_owned(),
        "Basic bGVtb25maWJlcjpkZWZnaGlqa2xtbm9wcXJz".to_owned()
    )));
    assert_eq!(
        one.body.as_deref(),
        Some(r#"{"method":"version","params":[]}"#)
    );
}

#[tokio::test]
async fn it_answers_only_while_its_version_is_answered_to_the_account() {
    for down in [
        Fake::silent(),
        Fake::always(Answer::reply(401, "")),
        Fake::always(Answer::reply(503, "")),
    ] {
        assert!(!Nzbget::new(down, BASE, Arc::new(account())).answers().await);
    }
    let up = Fake::always(Answer::reply(200, answer(r#""26.3""#)));
    assert!(
        !Nzbget::new(up.clone(), BASE, Arc::new(Nothing))
            .answers()
            .await
    );
    assert!(up.requests().is_empty());
}

#[tokio::test]
async fn without_an_account_nothing_is_asked_and_the_refusal_says_why() {
    let http = Fake::silent();
    let refused = Nzbget::new(http.clone(), BASE, Arc::new(Nothing))
        .pulling()
        .await;
    assert!(
        matches!(&refused, Err(Failure::Refused { detail, .. })
            if detail == "this adapter holds no account for NZBGet: upstream.json could not be read: not found"),
        "{refused:?}"
    );
    assert!(http.requests().is_empty());
}

#[tokio::test]
async fn an_account_nzbget_refuses_is_unauthorised_and_blamed_on_it() {
    let (nzbget, _) = client(vec![(401, "")]);
    assert!(matches!(
        nzbget.pulling().await,
        Err(Failure::Unauthorised { service }) if service == "nzbget"
    ));
}

#[tokio::test]
async fn an_error_nzbget_answers_is_a_refusal_in_its_own_words() {
    let (nzbget, _) = client(vec![(
        200,
        r#"{"version":"1.1","error":{"name":"JSONRPCError","code":1,"message":"Invalid procedure"}}"#,
    )]);
    let refused = nzbget.pulling().await;
    assert!(
        matches!(&refused, Err(Failure::Refused { detail, .. })
            if detail == "whether it is fetching could not be read: Invalid procedure"),
        "{refused:?}"
    );
}

#[tokio::test]
async fn an_answer_with_neither_a_result_nor_an_error_is_a_refusal() {
    let (nzbget, _) = client(vec![(200, r#"{"version":"1.1"}"#)]);
    let refused = nzbget.pulling().await;
    assert!(
        matches!(&refused, Err(Failure::Refused { detail, .. })
            if detail == "whether it is fetching could not be read: the client answered nothing"),
        "{refused:?}"
    );
}

#[test]
fn a_size_in_two_halves_is_joined_and_one_too_large_is_held_at_the_most() {
    assert_eq!(bytes(500_000, 0), 500_000);
    assert_eq!(bytes(1, 1), (1 << 32) + 1);
    assert_eq!(bytes(0, 1 << 32), u64::MAX);
    assert_eq!(bytes(u64::MAX, u64::from(u32::MAX)), u64::MAX);
}
