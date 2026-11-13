use async_trait::async_trait;
use lemonfiber_ports::service::{Failure, Rates, Throttled, Throttling, Wanted, Window};
use serde::Deserialize;
use serde_json::json;

use super::scheduling::{keeping, keeps, side, Turn};
use super::settings::{kilobytes, DOWNLOAD_RATE};
use super::{Nzbget, STATUS};

const LAST_HOUR: u8 = 23;

const LAST_MINUTE: u8 = 59;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Limited {
    #[serde(default)]
    download_limit: u64,
    #[serde(default)]
    download_rate: u64,
}

#[async_trait]
impl Throttling for Nzbget {
    async fn throttled(&self) -> Result<Throttled, Failure> {
        let held: Limited = self
            .call(STATUS, json!([]), "the rate limit could not be read")
            .await?;
        let running = self.live("the schedule could not be read").await?;
        let down = Some(held.download_limit).filter(|limit| *limit > 0);
        Ok(Throttled {
            rates: Rates { down, up: None },
            uploads: false,
            hours: side(&running, down.is_some()),
        })
    }

    async fn restrain(&self, wanted: &Wanted) -> Result<Throttled, Failure> {
        if !wanted.window.is_none_or(on_the_clock) {
            return Err(self
                .endpoint
                .refused("that window is not hours and minutes of a day"));
        }
        let turns = turns(wanted);
        let figure = kilobytes(wanted.active.down);
        let saved = self.saved("the settings could not be read").await?;
        let mut next = saved.clone();
        if turns.is_empty() {
            next.set(DOWNLOAD_RATE, &figure.to_string());
        }
        if keeps(&saved, &turns) {
            if next != saved {
                self.save(&next).await?;
            }
            if turns.is_empty() {
                self.asked("rate", json!([figure]), "the rate limit could not be set")
                    .await?;
            }
        } else {
            keeping(&mut next, &turns);
            self.reloaded(next, false, &|running| keeps(running, &turns))
                .await?;
        }
        self.throttled().await
    }

    async fn moving(&self) -> Result<Rates, Failure> {
        let held: Limited = self
            .call(
                STATUS,
                json!([]),
                "the current transfer rate could not be read",
            )
            .await?;
        Ok(Rates {
            down: Some(held.download_rate),
            up: None,
        })
    }
}

fn turns(wanted: &Wanted) -> Vec<Turn> {
    let Some(window) = wanted.window else {
        return Vec::new();
    };
    let (active, quiet) = (kilobytes(wanted.active.down), kilobytes(wanted.quiet.down));
    if active == quiet {
        return Vec::new();
    }
    vec![
        Turn {
            hour: window.from_hour,
            minute: window.from_minute,
            figure: active,
        },
        Turn {
            hour: window.to_hour,
            minute: window.to_minute,
            figure: quiet,
        },
    ]
}

fn on_the_clock(window: Window) -> bool {
    window.from_hour <= LAST_HOUR
        && window.to_hour <= LAST_HOUR
        && window.from_minute <= LAST_MINUTE
        && window.to_minute <= LAST_MINUTE
}

#[cfg(test)]
mod tests;
