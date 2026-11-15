use lemonfiber_ports::service::Hours;

use super::settings::{Entry, Settings};

pub(super) const TASK: &str = "Task";

const TIME: &str = "Time";

const WEEK_DAYS: &str = "WeekDays";

const COMMAND: &str = "Command";

const PARAM: &str = "Param";

const RATE: &str = "DownloadRate";

const EVERY_DAY: &str = "1-7";

/// One turn of the household's day: from `hour`:`minute`, the client held to `figure`
/// kilobytes a second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Turn {
    pub(super) hour: u8,
    pub(super) minute: u8,
    pub(super) figure: u64,
}

impl Turn {
    fn entry(&self) -> Entry {
        Entry::of(&[
            (TIME, &format!("{:02}:{:02}", self.hour, self.minute)),
            (WEEK_DAYS, EVERY_DAY),
            (COMMAND, RATE),
            (PARAM, &self.figure.to_string()),
        ])
    }
}

fn is_rate(task: &Entry) -> bool {
    task.get(COMMAND).eq_ignore_ascii_case(RATE)
}

fn written(task: &Entry) -> (String, String, String) {
    (
        task.get(TIME).trim().to_owned(),
        task.get(WEEK_DAYS).trim().to_owned(),
        task.get(PARAM).trim().to_owned(),
    )
}

/// Whether the rate instructions `settings` hold are `turns`, and no others.
pub(super) fn keeps(settings: &Settings, turns: &[Turn]) -> bool {
    let mut held: Vec<_> = settings
        .numbered(TASK)
        .iter()
        .filter(|task| is_rate(task))
        .map(written)
        .collect();
    let mut wanted: Vec<_> = turns.iter().map(|turn| written(&turn.entry())).collect();
    held.sort_unstable();
    wanted.sort_unstable();
    held == wanted
}

/// `settings` with its rate instructions replaced by `turns`, every other task kept.
pub(super) fn keeping(settings: &mut Settings, turns: &[Turn]) {
    let mut tasks: Vec<Entry> = settings
        .numbered(TASK)
        .into_iter()
        .filter(|task| !is_rate(task))
        .collect();
    tasks.extend(turns.iter().map(Turn::entry));
    settings.renumber(TASK, &tasks);
}

/// Which side of the household's day the client's scheduler has it on, where it holds
/// rate instructions with more than one figure.
pub(super) fn side(settings: &Settings, limited: bool) -> Option<Hours> {
    let mut figures: Vec<String> = settings
        .numbered(TASK)
        .iter()
        .filter(|task| is_rate(task))
        .map(|task| task.get(PARAM).trim().to_owned())
        .collect();
    figures.sort_unstable();
    figures.dedup();
    (figures.len() > 1).then_some(if limited { Hours::Active } else { Hours::Quiet })
}

#[cfg(test)]
mod tests;
