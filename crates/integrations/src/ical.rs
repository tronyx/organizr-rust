//! Calendars in the iCalendar format (`.ics`), for the calendar of the homepage.
//!
//! The reader takes what the PHP application takes from a calendar: the events
//! (`VEVENT`) with their start, their end, their title and their rule for repeats.
//! It gives the events in the form of the PHP application.

use std::ops::Range;

use jiff::civil::{Date, DateTime, Time, Weekday};
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};
use serde_json::{Value, json};

use crate::http::Outbound;
use crate::probe;

/// The seconds of a day.
const DAY: i64 = 86_400;

/// The largest calendar file that is read, in bytes.
const LARGEST: usize = 10 * 1024 * 1024;

/// The most periods of a rule for repeats that are looked at.
const MOST_PERIODS: i64 = 100_000;

/// The days around today that the calendar shows.
#[derive(Debug, Clone)]
pub struct Window {
    /// This moment.
    pub now: Timestamp,
    /// The time zone of the server. A time of a calendar that names no time zone is
    /// a time of this one.
    pub zone: TimeZone,
    /// The `calendarStart` setting: the days before today.
    pub days_before: i64,
    /// The `calendarEnd` setting: the days after today.
    pub days_after: i64,
}

impl Window {
    fn oldest(&self) -> i64 {
        self.now
            .as_second()
            .saturating_sub(self.days_before.saturating_mul(DAY))
    }

    fn newest(&self) -> i64 {
        self.now
            .as_second()
            .saturating_add(self.days_after.saturating_mul(DAY))
    }

    /// Whether the calendar shows an event that starts at the given moment. As in
    /// the PHP application, an event less than one day outside the days of the
    /// settings still shows.
    pub fn holds(&self, start: Timestamp) -> bool {
        let start = start.as_second();
        start > self.oldest().saturating_sub(DAY) && start < self.newest().saturating_add(DAY)
    }
}

/// Loads a calendar from an address. An address that starts with `webcal://` is
/// asked over HTTPS.
///
/// The text of an error names no address, because the address of a private calendar
/// holds a key.
pub async fn fetch(http: &Outbound, address: &str) -> Result<String, String> {
    let address = address.trim();
    let url = match address.split_at_checked(9) {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("webcal://") => {
            format!("https://{rest}")
        }
        _ => address.to_owned(),
    };
    let bytes = probe::download(http, &url, LARGEST).await?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// A date or a time of a calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stamp {
    /// A day without a time.
    Day(Date),
    /// A time of a time zone that the calendar names elsewhere, or of none.
    Local(DateTime),
    /// A time in UTC.
    Utc(DateTime),
}

/// A number that is written with digits only.
fn digits(text: &str, range: Range<usize>) -> Option<i8> {
    let part = text.get(range)?;
    if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

/// Reads `20201231`, `20201231T180000` or `20201231T180000Z`.
fn stamp(value: &str) -> Option<Stamp> {
    let value = value.trim();
    let year = i16::from(digits(value, 0..2)?) * 100 + i16::from(digits(value, 2..4)?);
    let date = Date::new(year, digits(value, 4..6)?, digits(value, 6..8)?).ok()?;
    if value.len() == 8 {
        return Some(Stamp::Day(date));
    }
    if !value.get(8..9)?.eq_ignore_ascii_case("T") {
        return None;
    }
    let second = if value.len() >= 15 {
        digits(value, 13..15)?
    } else {
        0
    };
    let time = Time::new(digits(value, 9..11)?, digits(value, 11..13)?, second, 0).ok()?;
    let moment = date.to_datetime(time);
    match value.get(15..).unwrap_or_default() {
        "" => Some(Stamp::Local(moment)),
        "Z" | "z" => Some(Stamp::Utc(moment)),
        _ => None,
    }
}

/// One line of a calendar: `NAME;PARAMETERS:VALUE`.
struct Property<'a> {
    name: &'a str,
    parameters: &'a str,
    value: &'a str,
}

fn property(line: &str) -> Option<Property<'_>> {
    // A colon inside quotation marks belongs to a parameter.
    let mut quoted = false;
    let colon = line.char_indices().find_map(|(place, character)| {
        if character == '"' {
            quoted = !quoted;
        }
        (character == ':' && !quoted).then_some(place)
    })?;
    let (head, value) = (line.get(..colon)?, line.get(colon + 1..)?);
    let (name, parameters) = head.split_once(';').unwrap_or((head, ""));
    Some(Property {
        name,
        parameters,
        value,
    })
}

/// The time zone that the parameters of a line name.
fn zone_parameter(parameters: &str) -> Option<String> {
    parameters.split(';').find_map(|parameter| {
        let (key, value) = parameter.split_once('=')?;
        key.eq_ignore_ascii_case("TZID")
            .then(|| value.trim_matches('"').to_owned())
    })
}

/// A text of a calendar without its escape characters.
fn plain_text(value: &str) -> String {
    let mut text = String::with_capacity(value.len());
    let mut characters = value.trim().chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            text.push(character);
            continue;
        }
        match characters.next() {
            Some('n' | 'N') => text.push(' '),
            Some(other) => text.push(other),
            None => {}
        }
    }
    text
}

/// An event as the calendar writes it.
#[derive(Debug, Default)]
struct Entry {
    start: Option<(Stamp, Option<String>)>,
    end: Option<(Stamp, Option<String>)>,
    title: Option<String>,
    rule: Option<String>,
}

/// What a calendar file holds.
#[derive(Debug, Default)]
struct Calendar {
    /// The `X-WR-TIMEZONE` of the calendar.
    zone: Option<String>,
    entries: Vec<Entry>,
}

fn parse(text: &str) -> Calendar {
    // A line that starts with a space goes on the line before it.
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        match (line.strip_prefix([' ', '\t']), lines.last_mut()) {
            (Some(more), Some(last)) => last.push_str(more),
            _ => lines.push(line.to_owned()),
        }
    }

    let mut calendar = Calendar::default();
    // The parts that are open, for example an alarm in an event in the calendar.
    let mut open: Vec<String> = Vec::new();
    let mut entry: Option<Entry> = None;
    for line in &lines {
        let Some(property) = property(line) else {
            continue;
        };
        let name = property.name.trim().to_ascii_uppercase();
        let value = property.value.trim();
        match name.as_str() {
            "BEGIN" => {
                let part = value.to_ascii_uppercase();
                if part == "VEVENT" {
                    entry = Some(Entry::default());
                }
                open.push(part);
                continue;
            }
            "END" => {
                if open.pop().as_deref() == Some("VEVENT") {
                    calendar.entries.extend(entry.take());
                }
                continue;
            }
            "X-WR-TIMEZONE" if calendar.zone.is_none() => {
                calendar.zone = Some(value.trim_matches('"').to_owned());
                continue;
            }
            _ => {}
        }
        // A line of a part inside an event, for example of an alarm, says nothing
        // about the event.
        let (Some("VEVENT"), Some(entry)) = (open.last().map(String::as_str), entry.as_mut())
        else {
            continue;
        };
        let time = || Some((stamp(value)?, zone_parameter(property.parameters)));
        match name.as_str() {
            "DTSTART" => entry.start = time(),
            "DTEND" => entry.end = time(),
            "SUMMARY" => entry.title = Some(plain_text(value)),
            "RRULE" => entry.rule = Some(value.to_owned()),
            _ => {}
        }
    }
    calendar
}

/// The name of a time zone in the time zone database. The PHP application knows
/// these other names.
fn standard_name(name: &str) -> &str {
    match name {
        "CST" | "Central Time" | "Central Standard Time" => "America/Chicago",
        "CET" | "Central European Time" => "Europe/Berlin",
        "EST" | "Eastern Time" | "Eastern Standard Time" => "America/New_York",
        "MST" | "Mountain Time" | "Mountain Standard Time" => "America/Denver",
        "PST" | "Pacific Time" | "Pacific Standard Time" => "America/Los_Angeles",
        "AKST" | "Alaska Time" | "Alaska Standard Time" => "America/Anchorage",
        "HST" | "Hawaii Time" | "Hawaii Standard Time" => "Pacific/Honolulu",
        "China Time" | "China Standard Time" => "Asia/Shanghai",
        "IST" | "India Time" | "India Standard Time" => "Asia/Kolkata",
        "JST" | "Japan Time" | "Japan Standard Time" => "Asia/Tokyo",
        "WET"
        | "WEST"
        | "Western European Time"
        | "Western European Standard Time"
        | "Western European Summer Time"
        | "W. Europe Time"
        | "W. Europe Standard Time"
        | "W. Europe Summer Time" => "Europe/Lisbon",
        other => other,
    }
}

fn zone_named(name: &str) -> Option<TimeZone> {
    TimeZone::get(standard_name(name.trim().trim_matches('"'))).ok()
}

/// The time zones of a calendar.
struct Zones<'a> {
    /// The time zone that the calendar names for itself, when it is a known one.
    calendar: Option<TimeZone>,
    /// Whether the calendar names a time zone for itself.
    calendar_is_named: bool,
    server: &'a TimeZone,
}

impl Zones<'_> {
    /// The time zone of a time that is not in UTC: the one that its line names, or
    /// the one of the calendar, or the one of the server. A time with a name that
    /// is no time zone is a time in UTC, as in the PHP application.
    fn of(&self, named: Option<&str>) -> TimeZone {
        if let Some(zone) = named.and_then(zone_named) {
            return zone;
        }
        if let Some(zone) = &self.calendar {
            return zone.clone();
        }
        if named.is_some() || self.calendar_is_named {
            TimeZone::UTC
        } else {
            self.server.clone()
        }
    }
}

/// The start and the end of an event, as times of one time zone.
struct Times {
    start: DateTime,
    end: DateTime,
    zone: TimeZone,
    /// Whether the start is a day without a time.
    whole_start: bool,
    /// Whether the end is a day without a time.
    whole_end: bool,
}

fn times(entry: &Entry, zones: &Zones<'_>) -> Option<Times> {
    let resolve = |(stamp, named): &(Stamp, Option<String>)| match stamp {
        Stamp::Day(day) => (
            day.to_datetime(Time::midnight()),
            zones.server.clone(),
            true,
        ),
        Stamp::Local(moment) => (*moment, zones.of(named.as_deref()), false),
        Stamp::Utc(moment) => (*moment, TimeZone::UTC, false),
    };
    let (start, zone, whole_start) = resolve(entry.start.as_ref()?);
    let (end, whole_end) = match entry.end.as_ref().map(resolve) {
        Some((end, _, true)) => (end, true),
        Some((end, end_zone, false)) => {
            let moment = end.to_zoned_datetime(end_zone).ok()?;
            (moment.with_time_zone(zone.clone()).datetime(), false)
        }
        // An event without an end: a whole day ends on the next day.
        None if whole_start => (start.checked_add(Span::new().days(1)).ok()?, true),
        None => (start, false),
    };
    Some(Times {
        start,
        end,
        zone,
        whole_start,
        whole_end,
    })
}

/// How often an event repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// A rule for repeats (`RRULE`), with the parts that this reader knows.
#[derive(Debug, PartialEq, Eq)]
struct Rule {
    frequency: Frequency,
    /// Every how many days, weeks, months or years.
    interval: i64,
    /// The most repeats, with the first event.
    count: Option<usize>,
    /// The last moment that an event starts at.
    until: Option<DateTime>,
    /// The days of the week. The number says which one of the month, from 1 or from
    /// -1 for the last one, and 0 is for every one.
    days: Vec<(i8, Weekday)>,
    /// The days of the month, from 1 or from -1 for the last one.
    month_days: Vec<i8>,
}

fn weekday(text: &str) -> Option<(i8, Weekday)> {
    let text = text.trim();
    let (number, name) = text.split_at_checked(text.len().checked_sub(2)?)?;
    let day = match name.to_ascii_uppercase().as_str() {
        "MO" => Weekday::Monday,
        "TU" => Weekday::Tuesday,
        "WE" => Weekday::Wednesday,
        "TH" => Weekday::Thursday,
        "FR" => Weekday::Friday,
        "SA" => Weekday::Saturday,
        "SU" => Weekday::Sunday,
        _ => return None,
    };
    let nth = if number.is_empty() {
        0
    } else {
        number.parse().ok().filter(|nth| *nth != 0)?
    };
    Some((nth, day))
}

impl Rule {
    /// Reads a rule. Nothing for a rule with a part that this reader does not know:
    /// such an event then shows once.
    fn parse(text: &str, zone: &TimeZone) -> Option<Self> {
        let mut rule = Self {
            frequency: Frequency::Daily,
            interval: 1,
            count: None,
            until: None,
            days: Vec::new(),
            month_days: Vec::new(),
        };
        let mut has_frequency = false;
        for part in text.trim().split(';').filter(|part| !part.is_empty()) {
            let (key, value) = part.split_once('=')?;
            match key.trim().to_ascii_uppercase().as_str() {
                "FREQ" => {
                    rule.frequency = match value.trim().to_ascii_uppercase().as_str() {
                        "DAILY" => Frequency::Daily,
                        "WEEKLY" => Frequency::Weekly,
                        "MONTHLY" => Frequency::Monthly,
                        "YEARLY" => Frequency::Yearly,
                        _ => return None,
                    };
                    has_frequency = true;
                }
                "INTERVAL" => rule.interval = value.trim().parse().ok().filter(|n| *n >= 1)?,
                "COUNT" => rule.count = Some(value.trim().parse().ok()?),
                "UNTIL" => {
                    rule.until = Some(match stamp(value)? {
                        Stamp::Day(day) => day.at(23, 59, 59, 0),
                        Stamp::Local(moment) => moment,
                        Stamp::Utc(moment) => moment
                            .to_zoned_datetime(TimeZone::UTC)
                            .ok()?
                            .with_time_zone(zone.clone())
                            .datetime(),
                    });
                }
                "BYDAY" => {
                    for day in value.split(',') {
                        rule.days.push(weekday(day)?);
                    }
                }
                "BYMONTHDAY" => {
                    for day in value.split(',') {
                        let day = day.trim().parse().ok().filter(|day| *day != 0)?;
                        rule.month_days.push(day);
                    }
                }
                "WKST" => {}
                _ => return None,
            }
        }
        let has_nth = rule.days.iter().any(|(nth, _)| *nth != 0);
        let fits = match rule.frequency {
            Frequency::Daily | Frequency::Yearly => {
                rule.days.is_empty() && rule.month_days.is_empty()
            }
            Frequency::Weekly => !has_nth && rule.month_days.is_empty(),
            Frequency::Monthly => rule.days.is_empty() || rule.month_days.is_empty(),
        };
        (has_frequency && fits).then_some(rule)
    }

    /// The days of a month that the rule names.
    fn days_of_month(&self, month: Date, start: DateTime) -> Vec<i8> {
        let last = month.days_in_month();
        let mut found = Vec::new();
        for day in &self.month_days {
            let day = if *day > 0 { *day } else { last + day + 1 };
            if (1..=last).contains(&day) {
                found.push(day);
            }
        }
        for (nth, weekday) in &self.days {
            if *nth == 0 {
                found.extend((1..=last).filter(|day| {
                    Date::new(month.year(), month.month(), *day)
                        .is_ok_and(|date| date.weekday() == *weekday)
                }));
            } else if let Ok(date) = month.nth_weekday_of_month(*nth, *weekday) {
                found.push(date.day());
            }
        }
        if self.month_days.is_empty() && self.days.is_empty() && start.day() <= last {
            found.push(start.day());
        }
        found.sort_unstable();
        found.dedup();
        found
    }

    /// The starts in one period of the rule: the day, the week, the month or the
    /// year with the given number after the first one. Nothing when the period is
    /// later than every date.
    fn period(&self, start: DateTime, index: i64) -> Option<Vec<DateTime>> {
        let step = index.checked_mul(self.interval)?;
        let time = start.time();
        match self.frequency {
            Frequency::Daily => {
                let span = Span::new().try_days(step).ok()?;
                Some(vec![start.checked_add(span).ok()?])
            }
            Frequency::Weekly if self.days.is_empty() => {
                let span = Span::new().try_weeks(step).ok()?;
                Some(vec![start.checked_add(span).ok()?])
            }
            Frequency::Weekly => {
                let since_monday = Span::new().days(start.weekday().to_monday_zero_offset());
                let monday = start.date().checked_sub(since_monday).ok()?;
                let week = monday.checked_add(Span::new().try_weeks(step).ok()?).ok()?;
                let mut places: Vec<i8> = self
                    .days
                    .iter()
                    .map(|(_, day)| day.to_monday_zero_offset())
                    .collect();
                places.sort_unstable();
                places.dedup();
                places
                    .into_iter()
                    .map(|place| {
                        let day = week.checked_add(Span::new().days(place)).ok()?;
                        Some(day.to_datetime(time))
                    })
                    .collect()
            }
            Frequency::Monthly => {
                let span = Span::new().try_months(step).ok()?;
                let month = start.date().first_of_month().checked_add(span).ok()?;
                let days = self.days_of_month(month, start);
                Some(
                    days.into_iter()
                        .filter_map(|day| Date::new(month.year(), month.month(), day).ok())
                        .map(|day| day.to_datetime(time))
                        .collect(),
                )
            }
            Frequency::Yearly => {
                let year = i16::try_from(i64::from(start.year()).checked_add(step)?).ok()?;
                if year > 9999 {
                    return None;
                }
                // The 29th of February is in a leap year only.
                Some(
                    Date::new(year, start.month(), start.day())
                        .map(|day| day.to_datetime(time))
                        .into_iter()
                        .collect(),
                )
            }
        }
    }

    /// The starts of the event from `first` to `last`.
    fn starts(&self, start: DateTime, first: DateTime, last: DateTime) -> Vec<DateTime> {
        let mut found = Vec::new();
        let mut counted = 0_usize;
        for index in 0..MOST_PERIODS {
            let Some(candidates) = self.period(start, index) else {
                break;
            };
            for candidate in candidates {
                if candidate < start {
                    continue;
                }
                counted += 1;
                if self.count.is_some_and(|count| counted > count)
                    || self.until.is_some_and(|until| candidate > until)
                    || candidate > last
                {
                    return found;
                }
                if candidate >= first {
                    found.push(candidate);
                }
            }
        }
        found
    }
}

/// One event in the form of the PHP application, with the moments of its start and
/// its end.
fn event(
    title: &str,
    times: &Times,
    (start, end): (DateTime, DateTime),
    window: &Window,
    shown_in: &TimeZone,
) -> Option<(Timestamp, Timestamp, Value)> {
    let start_moment = start
        .to_zoned_datetime(times.zone.clone())
        .ok()?
        .timestamp();
    let end_zone = if times.whole_end {
        window.zone.clone()
    } else {
        times.zone.clone()
    };
    let end_moment = end.to_zoned_datetime(end_zone).ok()?.timestamp();
    let state = if window.now < end_moment {
        "text-info"
    } else {
        "text-success"
    };
    let day = |at: DateTime| at.date().strftime("%Y-%m-%d").to_string();
    let start_text = if times.whole_start {
        day(start)
    } else {
        let shown = start_moment.to_zoned_datetime(shown_in.clone());
        // A start at midnight is a day without a time, as in the PHP application.
        if shown.hour() == 0 && shown.minute() == 0 {
            day(shown.datetime())
        } else {
            shown.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string()
        }
    };
    let end_text = if times.whole_end {
        day(end)
    } else {
        let shown = end_moment.to_zoned_datetime(shown_in.clone());
        shown.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string()
    };
    let event = json!({
        "title": title,
        "imagetype": format!("calendar-o text-warning text-custom-calendar {state}"),
        "imagetypeFilter": "ical",
        "className": "bg-calendar calendar-item bg-custom-calendar",
        "start": start_text,
        "end": end_text,
        "bgColor": state.replace("text", "bg"),
    });
    Some((start_moment, end_moment, event))
}

/// The time of a time zone at a moment that is given in seconds.
fn time_at(second: i64, zone: &TimeZone) -> Option<DateTime> {
    let moment = Timestamp::new(second, 0).ok()?;
    Some(moment.to_zoned_datetime(zone.clone()).datetime())
}

/// The events of a calendar file that the window holds, in the form of the PHP
/// application. An event without a start or without a title is left out.
pub fn events(text: &str, window: &Window) -> Vec<Value> {
    let calendar = parse(text);
    let zones = Zones {
        calendar: calendar.zone.as_deref().and_then(zone_named),
        calendar_is_named: calendar.zone.is_some(),
        server: &window.zone,
    };
    // The times of the answer are in the time zone of the calendar, or else in the
    // one of the server.
    let shown_in = zones
        .calendar
        .clone()
        .unwrap_or_else(|| window.zone.clone());

    let mut all = Vec::new();
    for entry in &calendar.entries {
        let (Some(title), Some(times)) = (entry.title.as_deref(), times(entry, &zones)) else {
            continue;
        };
        let rule = entry
            .rule
            .as_deref()
            .and_then(|rule| Rule::parse(rule, &times.zone));
        let Some(rule) = rule else {
            let once = event(title, &times, (times.start, times.end), window, &shown_in);
            if let Some((start, end, event)) = once
                && end.as_second() >= window.oldest()
                && window.holds(start)
            {
                all.push(event);
            }
            continue;
        };
        let bounds = (
            time_at(window.oldest().saturating_sub(DAY), &times.zone),
            time_at(window.newest().saturating_add(DAY), &times.zone),
            times.end.since(times.start),
        );
        let (Some(first), Some(last), Ok(length)) = bounds else {
            continue;
        };
        for start in rule.starts(times.start, first, last) {
            let Ok(end) = start.checked_add(length) else {
                continue;
            };
            if let Some((moment, _, event)) = event(title, &times, (start, end), window, &shown_in)
                && window.holds(moment)
            {
                all.push(event);
            }
        }
    }
    all
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Weekday, date};
    use jiff::tz::TimeZone;
    use serde_json::{Value, json};

    use super::{Frequency, Rule, Stamp, Window, events, parse, plain_text, stamp, weekday};

    /// Fourteen days around noon of the 15th of January 2020, on a server in the
    /// given time zone.
    fn window(zone: &str) -> Window {
        Window {
            now: "2020-01-15T12:00:00Z".parse().expect("a moment"),
            zone: TimeZone::get(zone).expect("a known time zone"),
            days_before: 14,
            days_after: 14,
        }
    }

    fn calendar(lines: &[&str]) -> String {
        let mut all = vec!["BEGIN:VCALENDAR", "VERSION:2.0"];
        all.extend(lines);
        all.push("END:VCALENDAR");
        all.join("\r\n")
    }

    fn starts(events: &[Value]) -> Vec<&str> {
        events
            .iter()
            .map(|event| event["start"].as_str().unwrap_or("?"))
            .collect()
    }

    #[test]
    fn dates_and_times_are_read_in_their_three_forms() {
        assert_eq!(stamp("20200131"), Some(Stamp::Day(date(2020, 1, 31))));
        assert_eq!(
            stamp("20200131T180500"),
            Some(Stamp::Local(date(2020, 1, 31).at(18, 5, 0, 0)))
        );
        assert_eq!(
            stamp(" 20200131T180500Z\r"),
            Some(Stamp::Utc(date(2020, 1, 31).at(18, 5, 0, 0)))
        );
        for wrong in [
            "",
            "2020-01-31",
            "20201331",
            "20200131 180500",
            "20200131T1805xx",
            "20200131T180500+01",
        ] {
            assert_eq!(stamp(wrong), None, "{wrong}");
        }
    }

    #[test]
    fn event_has_the_form_of_the_php_application() {
        let text = calendar(&[
            "X-WR-TIMEZONE:Europe/Berlin",
            "BEGIN:VEVENT",
            "DTSTART:20200120T170000Z",
            "DTEND:20200120T183000Z",
            "SUMMARY:Dinner\\, with a long",
            "  title",
            "BEGIN:VALARM",
            "TRIGGER:-PT15M",
            "SUMMARY:An alarm",
            "END:VALARM",
            "END:VEVENT",
        ]);

        let events = events(&text, &window("UTC"));

        assert_eq!(
            events,
            [json!({
                "title": "Dinner, with a long title",
                "imagetype": "calendar-o text-warning text-custom-calendar text-info",
                "imagetypeFilter": "ical",
                "className": "bg-calendar calendar-item bg-custom-calendar",
                "start": "2020-01-20T18:00:00+01:00",
                "end": "2020-01-20T19:30:00+01:00",
                "bgColor": "bg-info",
            })]
        );
    }

    #[test]
    fn time_is_a_time_of_the_zone_that_its_line_or_its_calendar_names() {
        let event = |start: &str, more: &str| {
            let text = calendar(&[
                more,
                "BEGIN:VEVENT",
                start,
                "DTEND:20200110T200000Z",
                "SUMMARY:One",
                "END:VEVENT",
            ]);
            events(&text, &window("America/New_York"))
                .first()
                .map(|event| event["start"].as_str().unwrap_or("?").to_owned())
        };

        // The line names the zone. The answer is in the zone of the server.
        assert_eq!(
            event("DTSTART;TZID=Europe/Berlin:20200110T180000", "X-A:1").as_deref(),
            Some("2020-01-10T12:00:00-05:00")
        );
        // A name of Windows that the PHP application knows.
        assert_eq!(
            event(
                "DTSTART;TZID=\"Pacific Standard Time\":20200110T090000",
                "X-A:1"
            )
            .as_deref(),
            Some("2020-01-10T12:00:00-05:00")
        );
        // No zone at all: a time of the server.
        assert_eq!(
            event("DTSTART:20200110T090000", "X-A:1").as_deref(),
            Some("2020-01-10T09:00:00-05:00")
        );
        // The calendar names the zone, and the answer is in that zone.
        assert_eq!(
            event("DTSTART:20200110T090000", "X-WR-TIMEZONE:Asia/Tokyo").as_deref(),
            Some("2020-01-10T09:00:00+09:00")
        );
        // A name that is no time zone: a time in UTC.
        assert_eq!(
            event("DTSTART;TZID=Nowhere/Land:20200110T180000", "X-A:1").as_deref(),
            Some("2020-01-10T13:00:00-05:00")
        );
    }

    #[test]
    fn whole_day_and_midnight_are_days_without_a_time() {
        let text = calendar(&[
            "BEGIN:VEVENT",
            "DTSTART;VALUE=DATE:20200110",
            "DTEND;VALUE=DATE:20200111",
            "SUMMARY:A birthday",
            "END:VEVENT",
            "BEGIN:VEVENT",
            "DTSTART;VALUE=DATE:20200112",
            "SUMMARY:No end",
            "END:VEVENT",
            "BEGIN:VEVENT",
            "DTSTART:20200113T000000",
            "DTEND:20200113T010000",
            "SUMMARY:At midnight",
            "END:VEVENT",
        ]);

        let events = events(&text, &window("Europe/Berlin"));

        assert_eq!(starts(&events), ["2020-01-10", "2020-01-12", "2020-01-13"]);
        assert_eq!(events[0]["end"], "2020-01-11");
        assert_eq!(events[1]["end"], "2020-01-13");
        assert_eq!(events[2]["end"], "2020-01-13T01:00:00+01:00");
        // What is over has another colour.
        assert_eq!(events[0]["bgColor"], "bg-success");
        assert_eq!(
            events[0]["imagetype"],
            "calendar-o text-warning text-custom-calendar text-success"
        );
    }

    #[test]
    fn event_outside_the_days_of_the_settings_is_left_out() {
        let one = |day: &str| {
            let (start, end) = (
                format!("DTSTART:{day}T120000Z"),
                format!("DTEND:{day}T130000Z"),
            );
            let text = calendar(&["BEGIN:VEVENT", &start, &end, "SUMMARY:One", "END:VEVENT"]);
            events(&text, &window("UTC")).len()
        };

        assert_eq!(one("20200101"), 1);
        assert_eq!(one("20200129"), 1);
        assert_eq!(one("20191230"), 0);
        assert_eq!(one("20200131"), 0);
        // Without a title or without a start, an event is none.
        let text = calendar(&["BEGIN:VEVENT", "DTSTART:20200110T120000Z", "END:VEVENT"]);
        assert_eq!(events(&text, &window("UTC")).len(), 0);
        let text = calendar(&["BEGIN:VEVENT", "SUMMARY:One", "END:VEVENT"]);
        assert_eq!(events(&text, &window("UTC")).len(), 0);
        assert_eq!(
            events("<html>not a calendar</html>", &window("UTC")).len(),
            0
        );
    }

    fn repeated(start: &str, rule: &str) -> Vec<String> {
        let text = calendar(&[
            "BEGIN:VEVENT",
            &format!("DTSTART:{start}"),
            &format!("DTEND:{start}"),
            &format!("RRULE:{rule}"),
            "SUMMARY:Again",
            "END:VEVENT",
        ]);
        starts(&events(&text, &window("UTC")))
            .into_iter()
            .map(|start| start.get(..10).unwrap_or("?").to_owned())
            .collect()
    }

    #[test]
    fn repeated_event_shows_on_each_day_of_its_rule_in_the_window() {
        // Every week on the day of the start, from long before the window.
        assert_eq!(
            repeated("20190102T100000Z", "FREQ=WEEKLY"),
            [
                "2020-01-01",
                "2020-01-08",
                "2020-01-15",
                "2020-01-22",
                "2020-01-29"
            ]
        );
        // Every second week, on two days.
        assert_eq!(
            repeated("20200106T100000Z", "FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH"),
            ["2020-01-06", "2020-01-09", "2020-01-20", "2020-01-23"]
        );
        // The count is from the first event, and the end date holds.
        assert_eq!(
            repeated("20200110T100000Z", "FREQ=DAILY;COUNT=3"),
            ["2020-01-10", "2020-01-11", "2020-01-12"]
        );
        assert_eq!(
            repeated("20191201T100000Z", "FREQ=DAILY;COUNT=3"),
            Vec::<String>::new()
        );
        assert_eq!(
            repeated("20200110T100000Z", "FREQ=DAILY;UNTIL=20200112T235959Z"),
            ["2020-01-10", "2020-01-11", "2020-01-12"]
        );
        // Each month: on the day of the start, on named days, on the second Tuesday,
        // on the last Friday.
        assert_eq!(repeated("20190520T100000Z", "FREQ=MONTHLY"), ["2020-01-20"]);
        assert_eq!(
            repeated("20190520T100000Z", "FREQ=MONTHLY;BYMONTHDAY=5,-1"),
            ["2020-01-05"]
        );
        assert_eq!(
            repeated("20190520T100000Z", "FREQ=MONTHLY;BYDAY=2TU"),
            ["2020-01-14"]
        );
        assert_eq!(
            repeated("20190520T100000Z", "FREQ=MONTHLY;BYDAY=-1FR"),
            Vec::<String>::new()
        );
        // Each year.
        assert_eq!(repeated("19990118T100000Z", "FREQ=YEARLY"), ["2020-01-18"]);
        // A rule that the reader does not know: the event shows once.
        assert_eq!(
            repeated("20200110T100000Z", "FREQ=MONTHLY;BYSETPOS=1;BYDAY=MO"),
            ["2020-01-10"]
        );
    }

    #[test]
    fn repeated_event_keeps_its_time_of_day_when_the_clocks_change() {
        let text = calendar(&[
            "BEGIN:VEVENT",
            "DTSTART;TZID=Europe/Berlin:20200320T180000",
            "DTEND;TZID=Europe/Berlin:20200320T190000",
            "RRULE:FREQ=WEEKLY",
            "SUMMARY:Again",
            "END:VEVENT",
        ]);
        let mut window = window("UTC");
        window.now = "2020-03-27T12:00:00Z".parse().expect("a moment");
        window.days_before = 7;
        window.days_after = 7;

        let events = events(&text, &window);

        assert_eq!(
            starts(&events),
            [
                "2020-03-20T17:00:00+00:00",
                "2020-03-27T17:00:00+00:00",
                "2020-04-03T16:00:00+00:00",
            ]
        );
        assert_eq!(events[2]["end"], "2020-04-03T17:00:00+00:00");
    }

    #[test]
    fn rule_is_read_with_the_parts_that_the_reader_knows() {
        let rule = Rule::parse(
            "FREQ=MONTHLY;INTERVAL=2;COUNT=5;BYDAY=-1FR;WKST=SU;",
            &TimeZone::UTC,
        )
        .expect("a rule");
        assert_eq!(rule.frequency, Frequency::Monthly);
        assert_eq!(rule.interval, 2);
        assert_eq!(rule.count, Some(5));
        assert_eq!(rule.days, [(-1, Weekday::Friday)]);
        assert_eq!(weekday("mo"), Some((0, Weekday::Monday)));
        assert_eq!(weekday("+2TU"), Some((2, Weekday::Tuesday)));
        assert_eq!(weekday("XX"), None);
        for unknown in [
            "INTERVAL=2",
            "FREQ=HOURLY",
            "FREQ=DAILY;INTERVAL=0",
            "FREQ=DAILY;BYDAY=MO",
            "FREQ=WEEKLY;BYDAY=1MO",
            "FREQ=YEARLY;BYMONTH=5",
            "FREQ=DAILY;UNTIL=soon",
        ] {
            assert_eq!(Rule::parse(unknown, &TimeZone::UTC), None, "{unknown}");
        }
    }

    #[test]
    fn calendar_file_is_read_line_by_line() {
        let text = "BEGIN:VCALENDAR\nX-WR-TIMEZONE:\"Europe/Berlin\"\nBEGIN:VTIMEZONE\nTZID:Europe/Berlin\nBEGIN:STANDARD\nDTSTART:19701025T030000\nEND:STANDARD\nEND:VTIMEZONE\nBEGIN:VEVENT\nDTSTART;TZID=\"Europe/Berlin\";VALUE=DATE-TIME:20200110T180000\nSUMMARY;LANGUAGE=en:One\\; two\nRRULE:FREQ=DAILY\nEND:VEVENT\nEND:VCALENDAR\n";

        let calendar = parse(text);

        assert_eq!(calendar.zone.as_deref(), Some("Europe/Berlin"));
        assert_eq!(calendar.entries.len(), 1);
        let entry = &calendar.entries[0];
        assert_eq!(
            entry.start,
            Some((
                Stamp::Local(date(2020, 1, 10).at(18, 0, 0, 0)),
                Some("Europe/Berlin".to_owned())
            ))
        );
        assert_eq!(entry.end, None);
        assert_eq!(entry.title.as_deref(), Some("One; two"));
        assert_eq!(entry.rule.as_deref(), Some("FREQ=DAILY"));
        assert_eq!(plain_text("a\\nb\\\\c"), "a b\\c");
    }
}
