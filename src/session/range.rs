use super::timestamp::{Ts, TsErr};
use chrono::{NaiveDate, NaiveDateTime, ParseError};
use std::{ops, str::FromStr};

#[derive(Debug, PartialEq)]
pub enum Range {
    AllDay(ops::Range<NaiveDate>),
    Timed(ops::Range<NaiveDateTime>),
}

impl Range {
    pub fn start(&self) -> Ts {
        // match self {
        //     Range::AllDay(nd) => Ts::Date(nd.start),
        //     Range::Timed(dt) => Ts::Timed(dt.start),
        // }
        todo!()
    }
}

impl FromStr for Range {
    type Err = RangeErr;

    // #[rustfmt::skip]
    fn from_str(str: &str) -> Result<Range, RangeErr> {
        let mut parts = str.splitn(2, "-");

        // Build start
        let start = parts.next().expect("first");

        // Build end
        let end_part = parts.next().ok_or(RangeErr::MissingEndBound)?;
        let mut end = start.to_string();
        end.replace_range(start.len() - end_part.len().., end_part);
        Ok(if start.contains("_") {
            Range::Timed(date_time(start)?..date_time(&end)?)
        } else {
            Range::AllDay(date(start)?..date(&end)?)
        })
    }
}

fn date_time(str: &str) -> Result<NaiveDateTime, RangeErr> {
    NaiveDateTime::parse_from_str(
        &overlay(str, "XX/01/01_00:00:00", false),
        "%y/%m/%d_%H:%M:%S",
    )
    .map_err(RangeErr::Parse)
}

#[rustfmt::skip]
fn date(string: &str) -> Result<NaiveDate, RangeErr> {
    NaiveDate::parse_from_str(
        &overlay(string, "XX/01/01", false),
        "%y/%m/%d"
    )
    .map_err(RangeErr::Parse)
}

fn overlay(over: &str, base: &str, trailing: bool) -> String {
    // TODO: Add out of bounds error
    assert!(base.len() >= over.len());
    let mut base = base.to_string();
    if trailing {
        base.replace_range(base.len() - over.len().., over)
    } else {
        base.replace_range(..over.len(), over);
    }
    println!("!{}!", base);
    base
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum RangeErr {
    #[error("Missing end date/time")]
    MissingEndBound,
    #[error("Timestamp: {0}")]
    Timestamp(#[from] TsErr),
    #[error("Mismatch between start and end times")]
    BoundMismatch,
    #[error("End before start")]
    EndBeforeStart,

    #[error("Time parsing error")]
    Parse(#[from] ParseError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    #[test]
    fn range_from_str_all_day_full() {
        let start = date(2024, 12, 31);
        let end = date(2025, 1, 2);
        assert_eq!(
            "24/12/31-25/01/02".parse::<Range>(),
            Ok(Range::AllDay(start..end))
        );
    }

    #[test]
    fn range_from_str_timed_full() {
        let start = date_time(2024, 12, 31, 10, 30, 0);
        let end = date_time(2025, 1, 2, 15, 45, 0);
        assert_eq!(
            "24/12/31_10:30-25/01/02_15:45".parse(),
            Ok(Range::Timed(start..end))
        );
    }

    #[test]
    fn range_from_str_all_day_partial() {
        let start = date(2024, 12, 2);
        let end = date(2024, 12, 8);
        assert_eq!(
            "24/12/02-08".parse::<Range>(),
            Ok(Range::AllDay(start..end))
        );
    }

    #[test]
    fn range_from_str_timed_partial() {
        let start = date_time(2024, 10, 2, 10, 30, 0);
        let end = date_time(2024, 10, 2, 15, 45, 0);
        assert_eq!(
            "24/10/02_10:30-15:45".parse::<Range>(),
            Ok(Range::Timed(start..end))
        );
    }

    #[test]
    fn range_from_str_timed_hours() {
        let start = date_time(2024, 10, 2, 10, 0, 0);
        let end = date_time(2024, 10, 2, 15, 0, 0);
        assert_eq!(
            "24/10/02_10-15".parse::<Range>(),
            Ok(Range::Timed(start..end))
        );
    }

    #[test]
    fn range_from_str_end_before_start() {
        assert_eq!(
            "24/10/02-01".parse::<Range>(),
            Err(RangeErr::EndBeforeStart)
        );
        assert_eq!(
            "24/10/02_10:50-09:38".parse::<Range>(),
            Err(RangeErr::EndBeforeStart)
        );
    }

    #[test]
    fn range_from_str_bound_mismatch() {
        assert_eq!(
            "24/10/02-01_23:55".parse::<Range>(),
            Err(RangeErr::BoundMismatch)
        );
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn date_time(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> NaiveDateTime {
        let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
        let time = NaiveTime::from_hms_opt(h, min, s).unwrap();
        NaiveDateTime::new(date, time)
    }
}
