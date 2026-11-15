use async_trait::async_trait;
use lemonfiber_manifest::Date;
use lemonfiber_ports::service::{Failure, Metering, Moved};

use super::Nzbget;

const EVERY_ACCOUNT: u64 = 0;

#[async_trait]
impl Metering for Nzbget {
    async fn moved(&self, month: &str) -> Result<Moved, Failure> {
        let volumes = self.volumes().await?;
        let daily = volumes
            .iter()
            .find(|volume| volume.server_id == EVERY_ACCOUNT)
            .map(super::Volume::daily)
            .unwrap_or_default();
        Ok(Moved {
            down: pulled_in(&daily, month),
            up: 0,
            since_start: false,
        })
    }
}

fn pulled_in(daily: &[(Date, u64)], month: &str) -> u64 {
    daily
        .iter()
        .filter(|(day, _)| written(*day) == month.trim())
        .map(|(_, bytes)| *bytes)
        .fold(0, u64::saturating_add)
}

fn written(day: Date) -> String {
    format!("{:04}-{:02}", day.year, day.month)
}

#[cfg(test)]
mod tests;
