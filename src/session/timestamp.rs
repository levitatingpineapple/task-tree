use chrono::{
    DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc, offset::LocalResult,
};
use rrule::Tz;
use std::{
    fmt::{self, Display, Formatter},
    num::ParseIntError,
    str::FromStr,
};

#[derive(Debug, PartialEq)]
pub enum Ts {
    Date(NaiveDate),
    Timed(DateTime<Local>),
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum TsErr {
    #[error("Not a number: {0}")]
    NotNumber(#[from] ParseIntError),
    #[error("Invalid time")]
    InvalidDate,
    #[error("Invalid date")]
    InvalidTime,
    #[error("Ambiguous in timezone")]
    AmbiguousInTimezone,
    #[error("Invalid in timezone")]
    InvalidInTimezone,
}

impl Ts {
    // Expressed as Utc date with time defaulting to mid-night
    pub fn as_utc(&self) -> Result<DateTime<Tz>, TsErr> {
        Ok(match self {
            Ts::Date(nd) => {
                let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
                Utc.from_utc_datetime(&nd.and_time(midnight))
                    .with_timezone(&Tz::UTC)
            }
            Ts::Timed(ndt) => ndt.clone().with_timezone(&Tz::UTC),
        })
    }
}

impl FromStr for Ts {
    type Err = TsErr;

    /// Decodes session's timestamp from a string
    fn from_str(str: &str) -> Result<Ts, TsErr> {
        let mut parts = str.splitn(2, "_");
        let date = parse_date(parts.next().expect("first"))?;
        Ok(if let Some(time_str) = parts.next() {
            let ndt = NaiveDateTime::new(date, parse_time(time_str)?);
            let utc = local_utc(&ndt)?;
            Ts::Timed(utc)
        } else {
            Ts::Date(date)
        })
    }
}

impl Display for Ts {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Ts::Date(date) => {
                let mut formatted = date.format("%y/%m/%d").to_string();
                trim_sufixes(&mut formatted, "/01");
                write!(f, "{}", formatted)
            }
            Ts::Timed(datetime) => {
                let mut date_part = datetime.format("%y/%m/%d").to_string();
                let mut time_part = datetime.format("%H:%M:%S").to_string();
                trim_sufixes(&mut date_part, "/01");
                trim_sufixes(&mut time_part, ":00");
                write!(f, "{}_{}", date_part, time_part)
            }
        }
    }
}

/// Trims up to two suffixes, if they match the default patter
fn trim_sufixes(s: &mut String, default: &str) {
    for _ in 0..2 {
        if s.ends_with(default) {
            s.truncate(s.len() - default.len());
        } else {
            return;
        }
    }
}

/// Interprets `NaiveDateTime` as `Utc`.
/// Throws error if time is ambiguous or invalid due to winter/summer time switch
fn local_utc(ndt: &NaiveDateTime) -> Result<DateTime<Local>, TsErr> {
    match Local.from_local_datetime(ndt) {
        LocalResult::Single(single) => Ok(single),
        LocalResult::Ambiguous(_, _) => Err(TsErr::AmbiguousInTimezone),
        LocalResult::None => Err(TsErr::InvalidInTimezone),
    }
}

/// Decodes date from str - only year is required
/// Omitted values default to first month or day
fn parse_date(str: &str) -> Result<NaiveDate, TsErr> {
    let mut parts = str.splitn(3, "/");
    Ok(NaiveDate::from_ymd_opt(
        parts
            .next()
            .expect("first")
            .parse::<i32>()
            .map(|y| y + 2000)
            .map_err(TsErr::NotNumber)?,
        parts
            .next()
            .map(|m| m.parse::<u32>())
            .unwrap_or(Ok(01))
            .map_err(TsErr::NotNumber)?,
        parts
            .next()
            .map(|d| d.parse::<u32>())
            .unwrap_or(Ok(01))
            .map_err(TsErr::NotNumber)?,
    )
    .ok_or(TsErr::InvalidDate)?)
}

/// Decodes time from str - only hours are required
/// Omitted values default to zeroth second or minute
fn parse_time(str: &str) -> Result<NaiveTime, TsErr> {
    let mut parts = str.splitn(3, ":");
    Ok(NaiveTime::from_hms_opt(
        parts
            .next()
            .expect("first")
            .parse::<u32>()
            .map_err(TsErr::NotNumber)?,
        parts
            .next()
            .map(|m| m.parse::<u32>())
            .unwrap_or(Ok(00))
            .map_err(TsErr::NotNumber)?,
        parts
            .next()
            .map(|s| s.parse::<u32>())
            .unwrap_or(Ok(00))
            .map_err(TsErr::NotNumber)?,
    )
    .ok_or(TsErr::InvalidTime)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_date_full() {
        assert_eq!(
            parse_date("24/12/31"),
            Ok(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap())
        )
    }

    #[test]
    fn parse_date_default_day() {
        assert_eq!(
            parse_date("24/12"),
            Ok(NaiveDate::from_ymd_opt(2024, 12, 1).unwrap())
        )
    }

    #[test]
    fn parse_date_default_month_day() {
        assert_eq!(
            parse_date("24"),
            Ok(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap())
        )
    }

    #[test]
    fn parse_date_invalid_format() {
        assert!(matches!(parse_date("not-a-date"), Err(TsErr::NotNumber(_))));
    }

    #[test]
    fn parse_date_invalid_date() {
        assert_eq!(parse_date("24/13/32"), Err(TsErr::InvalidDate));
    }

    #[test]
    fn parse_time_full() {
        assert_eq!(
            parse_time("14:30:45"),
            Ok(NaiveTime::from_hms_opt(14, 30, 45).unwrap())
        )
    }

    #[test]
    fn parse_time_default_second() {
        assert_eq!(
            parse_time("14:30"),
            Ok(NaiveTime::from_hms_opt(14, 30, 0).unwrap())
        )
    }

    #[test]
    fn parse_time_default_minute_second() {
        assert_eq!(
            parse_time("14"),
            Ok(NaiveTime::from_hms_opt(14, 0, 0).unwrap())
        )
    }

    #[test]
    fn parse_time_invalid_format() {
        assert!(matches!(parse_time("not-a-time"), Err(TsErr::NotNumber(_))));
    }

    #[test]
    fn parse_time_invalid_time() {
        assert_eq!(parse_time("25:70:99"), Err(TsErr::InvalidTime));
    }

    #[test]
    fn dt_from_str_date_only() {
        let expected_date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        assert_eq!("24/12/31".parse(), Ok(Ts::Date(expected_date)));
    }

    #[test]
    fn dt_from_str_with_time() {
        let expected_date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let expected_time = NaiveTime::from_hms_opt(14, 30, 45).unwrap();
        let expected_dt = NaiveDateTime::new(expected_date, expected_time);
        assert_eq!(
            "24/12/31_14:30:45".parse(),
            Ok(Ts::Timed(local_utc(&expected_dt).unwrap()))
        );
    }

    #[test]
    fn display() {
        let samples: Vec<&str> = vec!["24/08/02", "21/06_16", "22/01/02_13:51:34"];

        for sample in samples {
            if let Ok(ts) = Ts::from_str(sample) {
                assert_eq!(&ts.to_string(), sample);
            } else {
                assert!(false, "Failed constructing from sample");
            }
        }

        Ts::from_str("24/08/07").unwrap().to_string();
    }
}
