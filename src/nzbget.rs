//! NZBGet, the Usenet client, asked through its JSON-RPC API with the account lemonfiber
//! wrote for this adapter.

use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use lemonfiber_adapter_kit::endpoint::Endpoint;
use lemonfiber_manifest::Date;
use lemonfiber_ports::http::{Http, Method, Request};
use lemonfiber_ports::service::Failure;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::credential::{Credential, Holding};

mod accounts;
mod fetching;
mod filing;
mod metering;
mod queue;
mod scheduling;
mod settings;
mod throttling;

/// What every failure is blamed on.
pub const SERVICE: &str = "nzbget";

const SETTLING: Duration = Duration::from_millis(500);

const RPC: &str = "/jsonrpc";

const VERSION: &str = "version";

const STATUS: &str = "status";

const SERVER_VOLUMES: &str = "servervolumes";

const PAUSE: &str = "pausedownload";

const KILOBYTE: u64 = 1024;

const SECONDS_A_DAY: i64 = 86_400;

const HALF: u32 = 32;

/// A client for one NZBGet, signing in with the account `held` supplies on every call.
pub struct Nzbget {
    endpoint: Endpoint,
    held: Arc<dyn Holding>,
    settling: Duration,
}

impl Nzbget {
    /// The NZBGet reached at `base` over `http`, asked with the account `held` supplies.
    #[must_use]
    pub fn new(http: Arc<dyn Http>, base: impl Into<String>, held: Arc<dyn Holding>) -> Self {
        Self {
            endpoint: Endpoint::new(http, base, SERVICE),
            held,
            settling: SETTLING,
        }
    }

    /// The same client, left `settling` between two askings while NZBGet reloads.
    #[must_use]
    pub const fn settling(mut self, settling: Duration) -> Self {
        self.settling = settling;
        self
    }

    /// Whether NZBGet answers its version to the account this adapter holds.
    pub async fn answers(&self) -> bool {
        self.call::<String>(VERSION, json!([]), "its version could not be read")
            .await
            .is_ok()
    }

    fn credential(&self) -> Result<Credential, Failure> {
        self.held.credential().map_err(|why| {
            self.endpoint
                .refused(&format!("this adapter holds no account for NZBGet: {why}"))
        })
    }

    async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
        whenever: &str,
    ) -> Result<T, Failure> {
        let credential = self.credential()?;
        let request = Request {
            method: Method::Post,
            url: self.endpoint.url(RPC),
            headers: vec![
                ("Authorization".to_owned(), authorization(&credential)),
                ("Content-Type".to_owned(), "application/json".to_owned()),
            ],
            body: Some(json!({ "method": method, "params": params }).to_string()),
            pinned: None,
        };
        let response = self.endpoint.send(&request).await?;
        let answered: Answered<T> = self.endpoint.decode(&response, whenever)?;
        match (answered.result, answered.error) {
            (Some(result), None) => Ok(result),
            (_, Some(error)) => Err(self
                .endpoint
                .refused(&format!("{whenever}: {}", error.message))),
            (None, None) => Err(self
                .endpoint
                .refused(&format!("{whenever}: the client answered nothing"))),
        }
    }

    async fn asked(&self, method: &str, params: Value, whenever: &str) -> Result<(), Failure> {
        if self.call::<bool>(method, params, whenever).await? {
            Ok(())
        } else {
            Err(self.endpoint.refused(&format!(
                "{whenever}: the client answered that it would not"
            )))
        }
    }

    async fn volumes(&self) -> Result<Vec<Volume>, Failure> {
        self.call(
            SERVER_VOLUMES,
            json!([]),
            "what each account pulled could not be read",
        )
        .await
    }
}

fn authorization(credential: &Credential) -> String {
    let pair = format!("{}:{}", credential.username, credential.password);
    format!("Basic {}", STANDARD.encode(pair))
}

fn bytes(low: u64, high: u64) -> u64 {
    high.checked_shl(HALF)
        .filter(|shifted| shifted >> HALF == high)
        .map_or(u64::MAX, |shifted| shifted.saturating_add(low))
}

#[derive(Deserialize)]
struct Answered<T> {
    result: Option<T>,
    error: Option<Fault>,
}

#[derive(Deserialize)]
struct Fault {
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Volume {
    #[serde(rename = "ServerID")]
    server_id: u64,
    first_day: i64,
    #[serde(default)]
    total_size_lo: u64,
    #[serde(default)]
    total_size_hi: u64,
    #[serde(default)]
    bytes_per_days: Vec<Sized>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Sized {
    #[serde(default)]
    size_lo: u64,
    #[serde(default)]
    size_hi: u64,
}

impl Volume {
    fn total(&self) -> u64 {
        bytes(self.total_size_lo, self.total_size_hi)
    }

    fn daily(&self) -> Vec<(Date, u64)> {
        (self.first_day..)
            .zip(&self.bytes_per_days)
            .filter_map(|(day, size)| {
                let date = Date::from_unix_seconds(day.checked_mul(SECONDS_A_DAY)?)?;
                Some((date, bytes(size.size_lo, size.size_hi)))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
