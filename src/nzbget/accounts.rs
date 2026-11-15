use async_trait::async_trait;
use lemonfiber_ports::service::{Failure, UsenetAccount, UsenetAccounts};

use super::settings::Entry;
use super::{Nzbget, Volume};

const SERVER: &str = "Server";

const NAME: &str = "Name";

const HOST: &str = "Host";

const ACTIVE: &str = "Active";

const YES: &str = "yes";

#[async_trait]
impl UsenetAccounts for Nzbget {
    async fn accounts(&self) -> Result<Vec<UsenetAccount>, Failure> {
        let running = self.live("the Usenet accounts could not be read").await?;
        let volumes = self.volumes().await?;
        Ok((1_u64..)
            .zip(running.numbered(SERVER))
            .map(|(id, server)| {
                let pulled = volumes.iter().find(|volume| volume.server_id == id);
                account_of(&server, pulled)
            })
            .collect())
    }
}

fn account_of(server: &Entry, pulled: Option<&Volume>) -> UsenetAccount {
    let named = server.get(NAME).trim();
    let name = if named.is_empty() {
        server.get(HOST).trim()
    } else {
        named
    };
    UsenetAccount {
        name: name.to_owned(),
        enabled: server.get(ACTIVE).eq_ignore_ascii_case(YES),
        quota: None,
        downloaded: pulled.map_or(0, Volume::total),
        daily: pulled.map(Volume::daily).unwrap_or_default(),
        expires_on: None,
        standing: None,
    }
}

#[cfg(test)]
mod tests;
