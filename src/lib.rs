#![allow(unused_imports, unused_variables)]
use std::cmp::min;
use std::collections::HashSet;
use std::fmt::Debug;
use std::num::{ParseIntError, Saturating};
use std::ops::IndexMut;
use std::str::FromStr;
use std::string;

use chrono::format::ParseErrorKind;
use chrono::offset::LocalResult;
use chrono::{
    DateTime, Duration, Local, NaiveDate, NaiveTime, ParseError, ParseResult, TimeZone, Utc,
};
use chrono::{Datelike, NaiveDateTime};
use ics::ICalendar;
use ics::properties::*;
use rrule::{Frequency, Unvalidated, Validated};
use rrule::{RRule, Tz};

// Event

#[derive(Debug)]
struct Event {
    title: String,
    sessions: Vec<Session>,
}

impl FromStr for Event {
    type Err = String;

    fn from_str(_: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

// Session

#[derive(Debug, PartialEq)]
pub struct Session {
    start: DateTime<Local>,
    end: DateTime<Local>,
    repeat: Option<RRule>,
}

impl FromStr for Session {
    type Err = SessionFromStrError;

    fn from_str(str: &str) -> Result<Session, SessionFromStrError> {
        // Split
        let mut parts = str.splitn(2, "|");
        let mut range_parts = parts.next().expect("First").splitn(2, "-");
        let start_str = range_parts.next().expect("First");
        let end_str_suffix = range_parts.next().ok_or(SessionFromStrError::MissingEnd)?;
        let mut end_str = start_str.to_string();
        end_str.replace_range(start_str.len() - end_str_suffix.len().., end_str_suffix);
        // Parse
        let start = Session::local_date_time(start_str)?;
        Ok(Session {
            start,
            end: Session::local_date_time(&end_str)?,
            repeat: parts
                .next()
                .map(|part| {
                    println!("{}", part);
                    Session::repeat(part).and_then(|r| {
                        r.validate(start.with_timezone(&Tz::UTC))
                            .map_err(SessionFromStrError::InvalidRule)
                    })
                })
                .transpose()?,
        })
    }
}

impl Session {
    fn local_date_time(string: &str) -> Result<DateTime<Local>, SessionFromStrError> {
        let mut parts = string.splitn(2, "_");
        let date = NaiveDate::parse_from_str(parts.next().expect("First"), "%y/%m/%d")
            .map_err(SessionFromStrError::Format)?;
        let time = parts
            .next()
            .map(|str| {
                NaiveTime::parse_from_str(str, &"%H:%M:%S"[..min(str.len(), 8)])
                    .map_err(SessionFromStrError::Format)
            })
            .transpose()?
            .unwrap_or_default();
        match Local.from_local_datetime(&NaiveDateTime::new(date, time)) {
            LocalResult::Single(single) => Ok(single),
            LocalResult::Ambiguous(_, _) => Err(SessionFromStrError::AmbiguousLocalTime),
            LocalResult::None => Err(SessionFromStrError::InvalidLocalTime),
        }
    }

    fn repeat(str: &str) -> Result<RRule<Unvalidated>, SessionFromStrError> {
        let mut split = str.splitn(2, "-");
        let main = split.next().expect("First");
        let until = split
            .next()
            .map(|str| Session::local_date_time(str))
            .transpose()?;
        let mut parts = main.split("_");
        let mut rule = RRule::new(
            Frequency::from_str(parts.next().expect("First"))
                .map_err(SessionFromStrError::Frequency)?,
        );
        if let Some(until) = until {
            rule = rule.until(until.with_timezone(&Tz::UTC));
        }
        while let Some(part) = parts.next() {
            if let Some(prefix) = part.strip_prefix('%') {
                rule = rule.interval(
                    prefix
                        .parse::<u16>()
                        .map_err(SessionFromStrError::Interval)?,
                );
            } else if let Some(prefix) = part.strip_prefix("#") {
                rule = rule.count(prefix.parse::<u32>().map_err(SessionFromStrError::Count)?)
            }
        }
        Ok(rule)
    }
}

#[derive(Debug, PartialEq)]
pub enum SessionFromStrError {
    MissingEnd,
    Count(ParseIntError),
    Interval(ParseIntError),
    Frequency(rrule::ParseError),
    AmbiguousLocalTime,
    InvalidLocalTime,
    Format(chrono::ParseError),
    InvalidRule(rrule::RRuleError),
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime, SubsecRound};

    use super::*;

    #[test]
    fn session_from_str() {
        let start = Local.with_ymd_and_hms(2025, 03, 30, 21, 55, 0).unwrap();
        let repeat = assert_eq!(
            Session::from_str("25/03/30_21:55-23:11|daily"),
            Ok(Session {
                start: start,
                end: Local.with_ymd_and_hms(2025, 03, 30, 23, 11, 0).unwrap(),
                repeat: Some(
                    RRule::new(Frequency::Daily)
                        .validate(start.with_timezone(&Tz::UTC))
                        .unwrap()
                )
            })
        );
    }

    // #[test]
    fn session_ndt() {
        assert_eq!(
            Session::local_date_time("25/03/30_23:55:45"),
            Ok(Local.with_ymd_and_hms(2025, 03, 30, 23, 55, 45).unwrap())
        );

        assert_eq!(
            Session::local_date_time("25/03/30_23:55"),
            Ok(Local.with_ymd_and_hms(2025, 03, 30, 23, 55, 0).unwrap())
        );

        assert_eq!(
            Session::local_date_time("25/03/30"),
            Ok(Local.with_ymd_and_hms(2025, 03, 30, 0, 0, 0).unwrap())
        );

        // Test too long
        assert!(Session::local_date_time("25/03/30_23:55:45_TOO_LONG").is_err());

        // Test missing year
        assert!(Session::local_date_time("03/30_23:55").is_err());

        // Test that empty does not panic
        assert!(Session::local_date_time("").is_err());

        // Date part must be complete
        assert!(Session::local_date_time("25/03").is_err());
    }

    // #[test]
    fn session_repeat() {
        assert_eq!(Session::repeat("daily"), Ok(RRule::new(Frequency::Daily)));

        // Interval
        assert_eq!(
            Session::repeat("weekly_%3"),
            Ok(RRule::new(Frequency::Weekly).interval(3))
        );

        // Count
        assert_eq!(
            Session::repeat("daily_#42"),
            Ok(RRule::new(Frequency::Daily).count(42))
        );

        // Until
        let dt = Local
            .with_ymd_and_hms(2025, 03, 10, 0, 0, 0)
            .unwrap()
            .with_timezone(&Tz::UTC);
        assert_eq!(
            Session::repeat("monthly-25/03/10"),
            Ok(RRule::new(Frequency::Monthly).until(dt))
        );

        // Interval+Count
        assert_eq!(
            Session::repeat("yearly_%9_#42"),
            Ok(RRule::new(Frequency::Yearly).interval(9).count(42))
        );

        // Interval+Until
        assert_eq!(
            Session::repeat("yearly_%9-25/03/10"),
            Ok(RRule::new(Frequency::Yearly).interval(9).until(dt))
        );
    }
}
