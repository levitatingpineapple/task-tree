use super::timestamp::{Ts, TsErr};
use chrono::{NaiveDate, NaiveDateTime};
use std::str::FromStr;

#[derive(Debug, PartialEq, Clone)]
pub enum Range {
    AllDay(std::ops::Range<NaiveDate>),
    Timed(std::ops::Range<NaiveDateTime>),
}

#[derive(Debug, PartialEq)]
pub enum RangeErr {
    MissingEndBound,
    Timestamp(TsErr),
    BoundMismatch,
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
        let start = Ts::from_str(start_str).map_err(RangeErr::Timestamp)?;
        let end = Ts::from_str(&end_str).map_err(RangeErr::Timestamp)?;
        match start {
            Ts::Date(sd) => match end {
                Ts::Date(ed) => {
                    if sd <= ed {
                        Ok(Range::AllDay(sd..ed))
                    } else {
                        Err(RangeErr::EndBeforeStart)
                    }
                }
                Ts::DateTime(_) => Err(RangeErr::BoundMismatch),
            },
            Ts::DateTime(sdt) => match end {
                Ts::Date(_) => Err(RangeErr::BoundMismatch),
                Ts::DateTime(edt) => {
                    if sdt <= edt {
                        Ok(Range::Timed(sdt..edt))
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
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

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
        let start = NaiveDateTime::new(date(2024, 12, 31), time(10, 30, 0));
        let end = NaiveDateTime::new(date(2025, 1, 2), time(15, 45, 0));
        assert_eq!(
            "24/12/31_10:30-25/01/02_15:45".parse::<Range>(),
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
        let start = NaiveDateTime::new(date(2024, 10, 2), time(10, 30, 0));
        let end = NaiveDateTime::new(date(2024, 10, 2), time(15, 45, 0));
        assert_eq!(
            "24/10/02_10:30-15:45".parse::<Range>(),
            Ok(Range::Timed(start..end))
        );
    }

    #[test]
    fn range_from_str_timed_hours() {
        let start = NaiveDateTime::new(date(2024, 10, 2), time(10, 0, 0));
        let end = NaiveDateTime::new(date(2024, 10, 2), time(15, 0, 0));
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

    fn time(h: u32, m: u32, s: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, s).unwrap()
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }
}
