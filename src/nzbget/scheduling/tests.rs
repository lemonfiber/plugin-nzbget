use lemonfiber_ports::service::Hours;

use super::super::tests::settings;
use super::{keeping, keeps, side, Turn};

fn turns() -> Vec<Turn> {
    vec![
        Turn {
            hour: 7,
            minute: 0,
            figure: 1024,
        },
        Turn {
            hour: 23,
            minute: 30,
            figure: 0,
        },
    ]
}

const PAUSING: [(&str, &str); 3] = [
    ("Task1.Time", "02:00"),
    ("Task1.WeekDays", "1-7"),
    ("Task1.Command", "PauseDownload"),
];

#[test]
fn the_household_s_turns_replace_every_rate_instruction_and_keep_every_other_task() {
    let mut held = settings(&PAUSING);
    held.set("Task2.Time", "12:00");
    held.set("Task2.WeekDays", "1-5");
    held.set("Task2.Command", "downloadrate");
    held.set("Task2.Param", "50");
    keeping(&mut held, &turns());
    assert_eq!(
        held,
        settings(&[
            ("Task1.Time", "02:00"),
            ("Task1.WeekDays", "1-7"),
            ("Task1.Command", "PauseDownload"),
            ("Task2.Time", "07:00"),
            ("Task2.WeekDays", "1-7"),
            ("Task2.Command", "DownloadRate"),
            ("Task2.Param", "1024"),
            ("Task3.Time", "23:30"),
            ("Task3.WeekDays", "1-7"),
            ("Task3.Command", "DownloadRate"),
            ("Task3.Param", "0"),
        ])
    );
    assert!(keeps(&held, &turns()));
}

#[test]
fn a_schedule_keeps_the_turns_only_where_it_holds_them_and_no_others() {
    assert!(keeps(&settings(&PAUSING), &[]));
    assert!(!keeps(&settings(&PAUSING), &turns()));
    let mut extra = settings(&PAUSING);
    keeping(&mut extra, &turns());
    assert!(!keeps(&extra, &[]));
    assert!(!keeps(&extra, turns().get(..1).unwrap_or_default()));
}

#[test]
fn a_schedule_of_two_figures_puts_the_client_on_the_side_its_limit_says() {
    let mut held = settings(&PAUSING);
    assert_eq!(side(&held, true), None);
    keeping(&mut held, turns().get(..1).unwrap_or_default());
    assert_eq!(side(&held, true), None);
    keeping(&mut held, &turns());
    assert_eq!(side(&held, true), Some(Hours::Active));
    assert_eq!(side(&held, false), Some(Hours::Quiet));
}
