use super::timestamp::{Ts, TsErr};
use chrono::{DateTime, Local, NaiveDate};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub struct Range {
    pub start: Ts,
    pub end: Ts,
}

impl Range {
    fn all_day(start: NaiveDate, end: NaiveDate) -> Self {
        Self {
            start: Ts::Date(start),
            end: Ts::Date(end),
        }
    }

    fn timed(start: DateTime<Local>, end: DateTime<Local>) -> Self {
        Self {
            start: Ts::Timed(start),
            end: Ts::Timed(end),
        }
    }
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
}

impl FromStr for Range {
    type Err = RangeErr;

    fn from_str(str: &str) -> Result<Range, RangeErr> {
        let mut parts = str.splitn(2, "-");
        let start_str = parts.next().expect("first");
        let end_str_suffix = parts.next().ok_or(RangeErr::MissingEndBound)?;
        let mut end_str = start_str.to_string();
        end_str.replace_range(start_str.len() - end_str_suffix.len().., end_str_suffix);
        let start = Ts::from_str(start_str)?;
        let end = Ts::from_str(&end_str)?;
        match start {
            Ts::Date(sd) => match end {
                Ts::Date(ed) => {
                    if sd <= ed {
                        Ok(Range::all_day(sd, ed))
                    } else {
                        Err(RangeErr::EndBeforeStart)
                    }
                }
                Ts::Timed(_) => Err(RangeErr::BoundMismatch),
            },
            Ts::Timed(sdt) => match end {
                Ts::Date(_) => Err(RangeErr::BoundMismatch),
                Ts::Timed(edt) => {
                    if sdt <= edt {
                        Ok(Range::timed(sdt, edt))
                    } else {
                        Err(RangeErr::EndBeforeStart)
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone};

    #[test]
    fn range_from_str_all_day_full() {
        let start = date(2024, 12, 31);
        let end = date(2025, 1, 2);
        assert_eq!(
            "24/12/31-25/01/02".parse::<Range>(),
            Ok(Range::all_day(start, end))
        );
    }

    #[test]
    fn range_from_str_timed_full() {
        let start = Local.with_ymd_and_hms(2024, 12, 31, 10, 30, 0).unwrap();
        let end = Local.with_ymd_and_hms(2025, 1, 2, 15, 45, 0).unwrap();
        assert_eq!(
            "24/12/31_10:30-25/01/02_15:45".parse::<Range>(),
            Ok(Range::timed(start, end))
        );
    }

    #[test]
    fn range_from_str_all_day_partial() {
        let start = date(2024, 12, 2);
        let end = date(2024, 12, 8);
        assert_eq!(
            "24/12/02-08".parse::<Range>(),
            Ok(Range::all_day(start, end))
        );
    }

    #[test]
    fn range_from_str_timed_partial() {
        let start = Local.with_ymd_and_hms(2024, 10, 2, 10, 30, 0).unwrap();
        let end = Local.with_ymd_and_hms(2024, 10, 2, 15, 45, 0).unwrap();
        assert_eq!(
            "24/10/02_10:30-15:45".parse::<Range>(),
            Ok(Range::timed(start, end))
        );
    }

    #[test]
    fn range_from_str_timed_hours() {
        let start = Local.with_ymd_and_hms(2024, 10, 2, 10, 0, 0).unwrap();
        let end = Local.with_ymd_and_hms(2024, 10, 2, 15, 0, 0).unwrap();
        assert_eq!(
            "24/10/02_10-15".parse::<Range>(),
            Ok(Range::timed(start, end))
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
}
