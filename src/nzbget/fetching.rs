use async_trait::async_trait;
use lemonfiber_ports::service::{Failure, Fetching, Pulling};
use serde::Deserialize;
use serde_json::json;

use super::{Nzbget, PAUSE, STATUS};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Halted {
    #[serde(default)]
    download_paused: bool,
}

#[async_trait]
impl Fetching for Nzbget {
    async fn pulling(&self) -> Result<Pulling, Failure> {
        let held: Halted = self
            .call(
                STATUS,
                json!([]),
                "whether it is fetching could not be read",
            )
            .await?;
        let fetching = !held.download_paused;
        Ok(Pulling::of(fetching, fetching))
    }

    async fn stop(&self) -> Result<Pulling, Failure> {
        self.asked(PAUSE, json!([]), "the client could not be stopped")
            .await?;
        self.pulling().await
    }

    async fn resume(&self) -> Result<Pulling, Failure> {
        self.asked(
            "resumedownload",
            json!([]),
            "the client could not be started",
        )
        .await?;
        self.pulling().await
    }
}

#[cfg(test)]
mod tests;
