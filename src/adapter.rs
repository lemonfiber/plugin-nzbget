//! What this adapter says it is, and the kit that serves its contract.

use std::sync::Arc;

use lemonfiber_adapter_kit::{Kit, Readiness};
use lemonfiber_contract::adapter::{About, Release};
use lemonfiber_contract::capabilities::download;

use crate::nzbget::Nzbget;

/// Where NZBGet answers, by its service id and the port it listens on.
pub const ADDRESS: &str = "http://nzbget:6789";

/// The upstream this adapter fronts, as its project names itself.
pub const UPSTREAM: &str = "NZBGet";

/// The NZBGet release this adapter's recordings were taken from.
pub const RELEASE: &str = "v26.3-ls271";

/// The digest of the image that release's recordings were taken from.
pub const DIGEST: &str = "sha256:6f86d5cb17fa12c214e0c699addde8ee5b9622fbafd2f4dc2eb8f89e7251dc49";

/// What this adapter says it is: the contract it speaks and the release it supports.
#[must_use]
pub fn about() -> About {
    About {
        speaks: vec![lemonfiber_contract::spoken(
            download::usenet::CAPABILITY,
            download::usenet::MAJOR,
        )],
        upstream: UPSTREAM.to_owned(),
        releases: vec![Release {
            version: RELEASE.to_owned(),
            digest: DIGEST.to_owned(),
        }],
    }
}

/// The contract, served from `nzbget` under `key`, ready while NZBGet answers.
#[must_use]
pub fn kit(key: String, nzbget: Arc<Nzbget>) -> Kit {
    let asked = Arc::clone(&nzbget);
    let ready: Readiness = Arc::new(move || {
        let asked = Arc::clone(&asked);
        Box::pin(async move { asked.answers().await })
    });
    Kit::new(key, about())
        .ready_when(ready)
        .serving(download::usenet::served(nzbget))
}

#[cfg(test)]
mod tests;
