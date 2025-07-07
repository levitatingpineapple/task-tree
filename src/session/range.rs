use chrono::{
    DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, ParseError, TimeZone, Utc,
    offset::LocalResult,
};
use std::{ops, str::FromStr};

#[derive(Debug, PartialEq)]
pub enum Range {
    AllDay(ops::Range<NaiveDate>),
    Timed(ops::Range<DateTime<Local>>),
}

impl Range {
    pub fn start(&self) -> Bound {
        match &self {
            Range::AllDay(range) => Bound::AllDay(range.start),
            Range::Timed(range) => Bound::Timed(range.start),
        }
    }
}

impl FromStr for Range {
    type Err = RangeErr;

    fn from_str(str: &str) -> Result<Range, RangeErr> {
        let mut parts = str.splitn(2, "-");
        let start = parts.next().expect("first");
        let end = &overlay(
            parts.next().ok_or(RangeErr::MissingEndBound)?,
            start,
            Align::Trailing,
        )?;
        Ok(match Bound::from_str(start)? {
            Bound::AllDay(nd) => {
                let range = nd..date(end)?;
                if range.is_empty() {
                    return Err(RangeErr::Empty);
                }
                Range::AllDay(range)
            }
            Bound::Timed(dt) => {
                let range = dt..date_time(end)?;
                if range.is_empty() {
                    return Err(RangeErr::Empty);
                }
                Range::Timed(range)
            }
        })
    }
}

pub enum Bound {
    AllDay(NaiveDate),
    Timed(DateTime<Local>),
}

/// Represent start or end bound of the time `Range`
impl Bound {
    pub fn date_time(&self) -> DateTime<rrule::Tz> {
        match self {
            Bound::AllDay(nd) => Utc
                .from_utc_datetime(&nd.and_time(NaiveTime::default()))
                .with_timezone(&rrule::Tz::UTC),
            Bound::Timed(dt) => dt.with_timezone(&rrule::Tz::UTC),
        }
    }
}

impl FromStr for Bound {
    type Err = RangeErr;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        Ok(if str.contains("_") {
            Bound::Timed(date_time(str)?)
        } else {
            Bound::AllDay(date(str)?)
        })
    }
}

fn date_time(str: &str) -> Result<DateTime<Local>, RangeErr> {
    let overlay = overlay(str, "XX/01/01_00:00:00", Align::Leading);
    local(&NaiveDateTime::parse_from_str(&overlay?, "%y/%m/%d_%H:%M:%S").map_err(RangeErr::Parse)?)
}

fn date(str: &str) -> Result<NaiveDate, RangeErr> {
    let overlay = overlay(str, "XX/01/01", Align::Leading);
    NaiveDate::parse_from_str(&overlay?, "%y/%m/%d").map_err(RangeErr::Parse)
}

enum Align {
    Leading,
    Trailing,
}

/// Overlays a shorter string over base string
/// with leading or trailing alignment
fn overlay(over: &str, base: &str, align: Align) -> Result<String, RangeErr> {
    if base.len() >= over.len() {
        let mut base = base.to_string();
        match align {
            Align::Leading => base.replace_range(..over.len(), over),
            Align::Trailing => base.replace_range(base.len() - over.len().., over),
        }
        Ok(base)
    } else {
        Err(RangeErr::TooLong)
    }
}

/// Interprets `NaiveDateTime` as `Local`.
/// Throws error if time is ambiguous or invalid due to winter/summer time switch
pub fn local(ndt: &NaiveDateTime) -> Result<DateTime<Local>, RangeErr> {
    match Local.from_local_datetime(ndt) {
        LocalResult::Single(single) => Ok(single),
        LocalResult::Ambiguous(_, _) => Err(RangeErr::AmbiguousInTimezone),
        LocalResult::None => Err(RangeErr::InvalidInTimezone),
    }
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum RangeErr {
    #[error("Missing end date/time")]
    MissingEndBound,
    #[error("Empty or inverse range")]
    Empty,
    #[error("Time parsing error")]
    Parse(#[from] ParseError),
    #[error("Ambiguous in timezone")]
    AmbiguousInTimezone,
    #[error("Invalid in timezone")]
    InvalidInTimezone,
    #[error("End too long")]
    TooLong,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    #[test]
    fn all_day_partial() {
        let start = date(2024, 12, 2);
        let end = date(2024, 12, 8);
        assert_eq!(
            "24/12/02-08".parse::<Range>(),
            Ok(Range::AllDay(start..end))
        );
    }

    #[test]
    fn all_day_full() {
        let start = date(2024, 12, 31);
        let end = date(2025, 1, 2);
        assert_eq!(
            "24/12/31-25/01/02".parse::<Range>(),
            Ok(Range::AllDay(start..end))
        );
    }

    #[test]
    fn timed_partial() {
        let start = date_time(2024, 10, 2, 10, 30, 0);
        let end = date_time(2024, 10, 2, 15, 45, 0);
        assert_eq!(
            "24/10/02_10:30-15:45".parse::<Range>(),
            Ok(Range::Timed(start..end))
        );
    }

    #[test]
    fn timed_hours() {
        let start = date_time(2024, 10, 2, 10, 0, 0);
        let end = date_time(2024, 10, 2, 15, 0, 0);
        assert_eq!(
            "24/10/02_10-15".parse::<Range>(),
            Ok(Range::Timed(start..end))
        );
    }

    #[test]
    fn end_before_start() {
        assert_eq!("24/10/02-01".parse::<Range>(), Err(RangeErr::Empty));
        assert_eq!(
            "24/10/02_10:50-09:38".parse::<Range>(),
            Err(RangeErr::Empty)
        );
    }

    #[test]
    fn timed_full() {
        let start = date_time(2024, 12, 31, 10, 30, 0);
        let end = date_time(2025, 1, 2, 15, 45, 0);
        assert_eq!(
            "24/12/31_10:30-25/01/02_15:45".parse(),
            Ok(Range::Timed(start..end))
        );
    }

    #[test]
    fn overlay_leading() {
        assert_eq!(
            overlay("abc", "123456", Align::Leading),
            Ok("abc456".to_string())
        );
    }

    #[test]
    fn overlay_trailing() {
        assert_eq!(
            overlay("abc", "123456", Align::Trailing),
            Ok("123abc".to_string())
        );
    }

    #[test]
    fn overlay_too_same_length() {
        assert_eq!(
            overlay("hello", "world", Align::Leading),
            Ok("hello".to_string())
        );
    }

    #[test]
    fn overlay_too_long_error() {
        assert_eq!(
            overlay("toolong", "short", Align::Leading),
            Err(RangeErr::TooLong)
        );
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn date_time(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Local> {
        let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
        let time = NaiveTime::from_hms_opt(h, min, s).unwrap();
        local(&NaiveDateTime::new(date, time)).unwrap()
    }
}
