use chrono::{
    DateTime, Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, ParseError, TimeZone, Timelike,
    Utc, offset::LocalResult,
};
use std::{
    fmt::{self, Display, Formatter},
    ops::{self},
    str::FromStr,
};

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

    pub fn end(&self) -> Bound {
        match &self {
            Range::AllDay(range) => Bound::AllDay(range.end),
            Range::Timed(range) => Bound::Timed(range.end),
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

impl Display for Range {
    /// Both `start` and `end` bounds are dependent on one-another
    /// - `start`
    ///   - Prefix -> year
    ///   - Suffix
    ///     - `AllDay` -> last non-default *start and end* trailing component.
    ///     - `Timed` -> same but start from `%h` hours
    /// - `end`
    ///   - Prefix -> first non-matching *start and end* leading component
    ///   - Suffix -> same as start
    #[allow(unused_variables)]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let separators: Vec<char> = vec!['/', '/', '_', ':', ':'];
        let defaults: Vec<u32> = vec![1, 1, 1, 0, 0, 0];
        let sc = self.start().components();
        let ec = self.end().components();
        let mut suffix = defaults
            .iter()
            .zip(sc.iter().zip(ec.iter()))
            .rposition(|(d, (s, e))| d != s || d != e)
            .unwrap();
        suffix = match self {
            Range::AllDay(_) => suffix,
            Range::Timed(_) => std::cmp::max(suffix, 3),
        };
        let prefix = sc.iter().zip(ec.iter()).position(|(s, e)| s != e).unwrap();
        write(f, sc, 0..=suffix)?;
        write!(f, "-")?;
        write(f, ec, prefix..=suffix)
    }
}

#[derive(Debug, PartialEq)]
pub enum Bound {
    AllDay(NaiveDate),
    Timed(DateTime<Local>),
}

/// Represents start or end bound of the time `Range`
/// Time is interpreted and valid in system's local timezone
/// String representation trims trailing suffix with default values
impl Bound {
    /// Returns time in UTC timezone
    pub fn date_time(&self) -> DateTime<rrule::Tz> {
        match self {
            Bound::AllDay(nd) => Utc
                .from_utc_datetime(&nd.and_time(NaiveTime::default()))
                .with_timezone(&rrule::Tz::UTC),
            Bound::Timed(dt) => dt.with_timezone(&rrule::Tz::UTC),
        }
    }

    /// Returns time bound components as a vector.
    /// Used for trimming default suffix or common prefix
    /// in `Range`s `Display` implementation
    pub fn components(&self) -> Vec<u32> {
        match &self {
            Bound::AllDay(nd) => vec![
                nd.year().try_into().expect("B.C. not supported"),
                nd.month(),
                nd.day(),
            ],
            Bound::Timed(dt) => vec![
                dt.year().try_into().expect("B.C. not supported"),
                dt.month(),
                dt.day(),
                dt.hour(),
                dt.minute(),
                dt.second(),
            ],
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

impl Display for Bound {
    /// Only used to render repeat-rule
    #[rustfmt::skip]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fn date_time_format(dt: &DateTime<Local>) -> &'static str {
            if dt.second() != 0 { return "%y/%m/%d_%H:%M:%S"; }
            if dt.minute() != 0 { return "%y/%m/%d_%H:%M"; }
            if dt.hour()   != 0 { return "%y/%m/%d_%H"; }
            date_format(dt)
        }
        fn date_format<T: Datelike>(date: &T) -> &'static str {
            if date.day()   != 1 { return "%y/%m/%d"; }
            if date.month() != 1 { return "%y/%m"; }
                                   return "%y";
        }
        write!(
            f,
            "{}",
            match &self {
                Bound::AllDay(nd) => nd.format(date_format(nd)),
                Bound::Timed(dt) => dt.format(date_time_format(dt)),
            }
        )
    }
}

/// Writes a range of bound components with provided formatter
/// Due to many range combinations this would be verbose to
/// implement in a type-safe way.
/// Expects 3 (AllDay) or 6 (Timed) components and range
/// within the bounds of the `components`
fn write(
    f: &mut Formatter<'_>,
    components: Vec<u32>,
    range: ops::RangeInclusive<usize>,
) -> fmt::Result {
    // Sanity check
    debug_assert!([3usize, 6usize].contains(&components.len()));
    debug_assert!(range.start() <= &components.len() && range.end() <= &components.len());
    let separators: Vec<char> = vec!['/', '/', '_', ':', ':'];
    let s = range.start().clone();
    for i in range {
        if i != s {
            write!(f, "{}", separators[i - 1])?; // leading separator
        }
        if i > 0 {
            write!(f, "{:02}", components[i])?; // other components
        } else {
            // NOTE: Events before 2000 not supported and will panic
            write!(f, "{:02}", components[i] - 2000)? // year
        }
    }
    Ok(())
}

/// Parses date time with omitted default suffix
fn date_time(str: &str) -> Result<DateTime<Local>, RangeErr> {
    let overlay = overlay(str, "XX/01/01_00:00:00", Align::Leading);
    local(&NaiveDateTime::parse_from_str(&overlay?, "%y/%m/%d_%H:%M:%S").map_err(RangeErr::Parse)?)
}

/// Parses date with omitted default suffix
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
    #[rustfmt::skip]
    fn range_display() {
        test(
            "25/08-09",
            Range::AllDay(d(2025, 8, 1)..d(2025, 9, 1))
        );
        test(
            "25/01/28-30",
            Range::AllDay(d(2025, 1, 28)..d(2025, 1, 30))
        );
        test(
            "25-28",
            Range::AllDay(d(2025, 1, 1)..d(2028, 1, 1))
        );
        test(
            "25/08/01_17-18",
            Range::Timed(dt(2025, 8, 1, 17, 0, 0)..dt(2025, 8, 1, 18, 0, 0))
        );
        test(
            "25/03/02_15:45-04/01_11:46",
            Range::Timed(dt(2025, 3, 2, 15, 45, 00)..dt(2025, 4, 1, 11, 46, 00))
        );

        fn test(str: &str, range: Range) {
            assert_eq!(&range.to_string(), str);
            assert_eq!(Range::from_str(str), Ok(range));
        }
    }

    #[test]
    #[rustfmt::skip]
    fn bound_display() {
        test(
            "25/07/31_19:45:58",
            Bound::Timed(dt(2025, 07, 31, 19, 45, 58))
        );
        test(
            "25/07/31_19:45",
            Bound::Timed(dt(2025, 07, 31, 19, 45, 00))
        );
        test(
            "25/07/31_19",
            Bound::Timed(dt(2025, 07, 31, 19, 00, 00))
        );
        test(
            "25/01/31",
            Bound::AllDay(d(2025, 01, 31))
        );
        test(
            "25/07",
            Bound::AllDay(d(2025, 07, 01))
        );
        test(
            "25",
            Bound::AllDay(d(2025, 01, 01))
        );

        fn test(str: &str, bound: Bound) {
            assert_eq!(&bound.to_string(), str);
            assert_eq!(Bound::from_str(str), Ok(bound));
        }
    }

    #[test]
    fn range_error() {
        unsafe {
            // Override system timezone
            std::env::set_var("TZ", "America/New_York");
        }
        test("25/07/08", RangeErr::MissingEndBound);
        test("25/07/08-07", RangeErr::Empty);
        test("25/03/09_02:30-40", RangeErr::InvalidInTimezone);
        test("25/11/02_01:30-40", RangeErr::AmbiguousInTimezone);

        fn test(str: &str, err: RangeErr) {
            assert_eq!(Range::from_str(str), Err(err));
        }
    }

    #[test]
    fn string_overlay() {
        assert_eq!(
            overlay("abc", "123456", Align::Leading),
            Ok("abc456".to_string())
        );
        assert_eq!(
            overlay("abc", "123456", Align::Trailing),
            Ok("123abc".to_string())
        );
        assert_eq!(
            overlay("hello", "world", Align::Leading),
            Ok("hello".to_string())
        );
        assert_eq!(
            overlay("toolong", "short", Align::Leading),
            Err(RangeErr::TooLong)
        );
    }

    fn d(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Local> {
        unsafe {
            // Override system timezone
            std::env::set_var("TZ", "UTC");
        }
        let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
        let time = NaiveTime::from_hms_opt(h, min, s).unwrap();
        local(&NaiveDateTime::new(date, time)).unwrap()
    }
}
