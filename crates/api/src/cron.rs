//! Cron schedules with the five fields of a cron table: minute, hour, day of the
//! month, month, day of the week.
//!
//! A field is `*`, a number, a range (`1-5`), a step (`*/5`, `10-30/5`, `3/10`), or a
//! list of these with commas. A month and a day of the week can be a name of three
//! letters (`jan`, `mon`). For the day of the week, 0 and 7 are both Sunday.
//!
//! As in a cron table, a schedule that limits the day of the month and the day of the
//! week names a day that fits one of the two.

use std::str::FromStr;

use jiff::ZonedDateTime;

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

const WEEK_DAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

/// The values that one field names, as one bit for each value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Field {
    values: u64,
    /// Whether the field is `*`, which limits nothing.
    is_any: bool,
}

impl Field {
    const fn holds(self, value: u32) -> bool {
        value < 64 && self.values & (1 << value) != 0
    }
}

/// A number of a field, or the place of a name in the list of its names.
fn number(text: &str, names: &[&str], first: u32) -> Option<u32> {
    if let Ok(number) = text.parse::<u32>() {
        return Some(number);
    }
    let lower = text.to_ascii_lowercase();
    names
        .iter()
        .position(|name| *name == lower)
        .and_then(|place| u32::try_from(place).ok())
        .map(|place| place + first)
}

/// Reads one field whose values run from `least` to `most`.
fn field(text: &str, least: u32, most: u32, names: &[&str]) -> Result<Field, String> {
    let wrong = || format!("`{text}` is no field from {least} to {most}");
    let mut values = 0_u64;
    for part in text.split(',') {
        let (range, step) = match part.split_once('/') {
            Some((range, step)) => (range, step.parse::<u32>().ok().filter(|step| *step > 0)),
            None => (part, Some(1)),
        };
        let step = step.ok_or_else(wrong)?;
        let (from, to) = if range == "*" {
            (least, most)
        } else if let Some((from, to)) = range.split_once('-') {
            (
                number(from, names, least).ok_or_else(wrong)?,
                number(to, names, least).ok_or_else(wrong)?,
            )
        } else {
            let from = number(range, names, least).ok_or_else(wrong)?;
            // A single number with a step runs to the end of the field.
            (from, if part.contains('/') { most } else { from })
        };
        if from < least || to > most || from > to {
            return Err(wrong());
        }
        let mut value = from;
        while value <= to {
            values |= 1 << value;
            value += step;
        }
    }
    Ok(Field {
        values,
        is_any: text == "*",
    })
}

/// A cron schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule {
    minute: Field,
    hour: Field,
    day: Field,
    month: Field,
    week_day: Field,
}

impl FromStr for Schedule {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        // The words that the settings of the PHP application use for a schedule.
        let text = match text.trim() {
            "@hourly" => "0 * * * *",
            "@daily" | "@midnight" => "0 0 * * *",
            "@weekly" => "0 0 * * 0",
            "@monthly" => "0 0 1 * *",
            "@yearly" | "@annually" => "0 0 1 1 *",
            other => other,
        };
        let fields: Vec<&str> = text.split_whitespace().collect();
        let [minute, hour, day, month, week_day] = fields.as_slice() else {
            return Err(format!(
                "a schedule has five fields, and `{text}` has {}",
                fields.len()
            ));
        };
        let mut week_day = field(week_day, 0, 7, &WEEK_DAYS)?;
        // 7 is Sunday, as 0 is.
        if week_day.holds(7) {
            week_day.values |= 1;
        }
        Ok(Self {
            minute: field(minute, 0, 59, &[])?,
            hour: field(hour, 0, 23, &[])?,
            day: field(day, 1, 31, &[])?,
            month: field(month, 1, 12, &MONTHS)?,
            week_day,
        })
    }
}

impl Schedule {
    /// Whether the schedule names the minute of a time.
    pub fn names(&self, time: &ZonedDateTime) -> bool {
        let part = |value: i8| u32::try_from(value).unwrap_or(u32::MAX);
        let week_day = u32::from(time.weekday().to_sunday_zero_offset().unsigned_abs());
        let day = self.day.holds(part(time.day()));
        let week_day = self.week_day.holds(week_day);
        let day_fits = match (self.day.is_any, self.week_day.is_any) {
            (false, false) => day || week_day,
            _ => day && week_day,
        };
        self.minute.holds(part(time.minute()))
            && self.hour.holds(part(time.hour()))
            && self.month.holds(part(time.month()))
            && day_fits
    }
}

#[cfg(test)]
mod tests {
    use jiff::ZonedDateTime;

    use super::Schedule;

    fn names(schedule: &str, time: &str) -> bool {
        let schedule: Schedule = schedule.parse().expect("the schedule is valid");
        let time: ZonedDateTime = format!("{time}[UTC]").parse().expect("the time is valid");
        schedule.names(&time)
    }

    // 20 November 2026 is a Friday.
    const FRIDAY_NOON: &str = "2026-11-20T12:00:00+00:00";

    #[test]
    fn fields_take_numbers_ranges_steps_and_lists() {
        assert!(names("* * * * *", FRIDAY_NOON));
        assert!(names("0 12 20 11 *", FRIDAY_NOON));
        assert!(!names("1 12 20 11 *", FRIDAY_NOON));
        assert!(names("*/15 * * * *", "2026-11-20T12:45:00+00:00"));
        assert!(!names("*/15 * * * *", "2026-11-20T12:50:00+00:00"));
        assert!(names("10-30/5 * * * *", "2026-11-20T12:25:00+00:00"));
        assert!(!names("10-30/5 * * * *", "2026-11-20T12:35:00+00:00"));
        // A number with a step runs to the end of the field.
        assert!(names("3/10 * * * *", "2026-11-20T12:53:00+00:00"));
        assert!(!names("3/10 * * * *", "2026-11-20T12:02:00+00:00"));
        assert!(names("0,30 8-18 * * *", "2026-11-20T18:30:00+00:00"));
        assert!(!names("0,30 8-18 * * *", "2026-11-20T19:30:00+00:00"));
    }

    #[test]
    fn months_and_week_days_have_names_and_sunday_is_0_and_7() {
        assert!(names("0 12 * nov fri", FRIDAY_NOON));
        assert!(names("0 12 * NOV FRI", FRIDAY_NOON));
        assert!(!names("0 12 * dec *", FRIDAY_NOON));
        assert!(names("0 12 * * 5", FRIDAY_NOON));
        assert!(names("0 12 * * mon-fri", FRIDAY_NOON));
        assert!(!names("0 12 * * sat,sun", FRIDAY_NOON));
        // 22 November 2026 is a Sunday.
        for sunday in ["0", "7", "sun"] {
            assert!(
                names(&format!("0 12 * * {sunday}"), "2026-11-22T12:00:00+00:00"),
                "{sunday}"
            );
        }
    }

    #[test]
    fn day_of_the_month_and_day_of_the_week_together_name_either() {
        // The 20th, or a Monday.
        assert!(names("0 12 20 * mon", FRIDAY_NOON));
        assert!(names("0 12 1 * fri", FRIDAY_NOON));
        assert!(!names("0 12 1 * mon", FRIDAY_NOON));
        // With one of them open, the other one decides.
        assert!(!names("0 12 1 * *", FRIDAY_NOON));
        assert!(!names("0 12 * * mon", FRIDAY_NOON));
    }

    #[test]
    fn words_stand_for_the_schedules_that_the_php_application_gives_them() {
        let parsed = |text: &str| text.parse::<Schedule>();
        for (word, fields) in [
            ("@hourly", "0 * * * *"),
            ("@daily", "0 0 * * *"),
            ("@midnight", "0 0 * * *"),
            ("@weekly", "0 0 * * 0"),
            ("@monthly", "0 0 1 * *"),
            ("@yearly", "0 0 1 1 *"),
            ("@annually", "0 0 1 1 *"),
            (" @weekly ", "0 0 * * sun"),
        ] {
            assert_eq!(parsed(word), parsed(fields), "{word}");
            assert!(parsed(word).is_ok(), "{word}");
        }
        assert!(names("@hourly", FRIDAY_NOON));
        assert!(!names("@daily", FRIDAY_NOON));
        // The week starts on Sunday: 22 November 2026.
        assert!(names("@weekly", "2026-11-22T00:00:00+00:00"));
        assert!(!names("@weekly", "2026-11-21T00:00:00+00:00"));
        assert!(parsed("@often").is_err());
        assert!(parsed("@daily *").is_err());
    }

    #[test]
    fn text_that_is_no_schedule_is_refused() {
        for wrong in [
            "",
            "often",
            "* * * *",
            "* * * * * *",
            "60 * * * *",
            "* 24 * * *",
            "* * 0 * *",
            "* * * 13 *",
            "* * * * 8",
            "*/0 * * * *",
            "5-1 * * * *",
            "a * * * *",
            "* * * * funday",
        ] {
            assert!(wrong.parse::<Schedule>().is_err(), "{wrong}");
        }
    }
}
