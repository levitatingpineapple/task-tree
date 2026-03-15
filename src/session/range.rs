use chrono::{
    DateTime, Datelike, Days, Months, NaiveDate, NaiveDateTime, NaiveTime, ParseError, TimeDelta,
    Timelike, Weekday, offset::LocalResult,
};
use chrono_tz::{GapInfo, Tz};
use std::{
    fmt::{self, Display, Formatter},
    ops::RangeInclusive,
    str::FromStr,
};

#[derive(Debug, PartialEq)]
pub enum Range {
    AllDay(Span<NaiveDate>),
    Timed(Span<DateTime<Tz>>),
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

    pub fn time_delta(&self) -> TimeDelta {
        self.end().dt() - self.start().dt()
    }

    pub fn month(year: i32, month: u32) -> Option<Range> {
        NaiveDate::from_ymd_opt(year, month, 1).map(|start| {
            let end = start
                .checked_add_months(Months::new(1))
                .expect("adding single month to valid date never overflows");
            Range::AllDay(Span::new(start, end))
        })
    }

    pub fn week(year: i32, week: u32) -> Option<Range> {
        NaiveDate::from_isoywd_opt(year, week, Weekday::Mon).map(|start| {
            let end = start
                .checked_add_days(Days::new(7))
                .expect("adding seven days to valid date never overflows");
            Range::AllDay(Span::new(start, end))
        })
    }

    pub fn into_dt_span(self) -> Span<DateTime<Tz>> {
        match self {
            Range::AllDay(span) => Span::new(first_time(&span.start), first_time(&span.end)),
            Range::Timed(span) => span,
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
                let span = Span::new(nd, date(end)?);
                if span.is_empty() {
                    return Err(RangeErr::Empty);
                }
                Range::AllDay(span)
            }
            Bound::Timed(dt) => {
                let span = Span::new(dt, date_time(end)?);
                if span.is_empty() {
                    return Err(RangeErr::Empty);
                }
                Range::Timed(span)
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span<T: PartialEq + Ord + Copy> {
    pub start: T,
    pub end: T,
}

impl<T: Ord + Copy> Span<T> {
    pub fn new(start: T, end: T) -> Span<T> {
        Span { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

#[derive(Debug, PartialEq)]
pub enum Bound {
    AllDay(NaiveDate),
    Timed(DateTime<Tz>),
}

/// Represents start or end bound of the time `Range`
/// Time is interpreted and valid in a given timezone
/// String representation trims trailing suffix with default values
impl Bound {
    pub fn dt(self) -> DateTime<Tz> {
        match self {
            Bound::AllDay(nd) => first_time(&nd),
            Bound::Timed(dt) => dt,
        }
    }

    /// Returns time bound components as a vector.
    /// Used for trimming default suffix or common prefix
    /// in `Range`s `Display` implementation
    fn components(&self) -> Vec<u32> {
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

/// Writes a range of bound components with provided formatter
/// Due to many range combinations this would be verbose to
/// implement in a type-safe way.
/// Expects 3 (AllDay) or 6 (Timed) components and range
/// within the bounds of the `components`
fn write(f: &mut Formatter<'_>, components: Vec<u32>, range: RangeInclusive<usize>) -> fmt::Result {
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
fn date_time(str: &str) -> Result<DateTime<Tz>, RangeErr> {
    let overlay = overlay(str, "XX/01/01_00:00:00", Align::Leading)?;
    in_timezone(&NaiveDateTime::parse_from_str(
        &overlay,
        "%y/%m/%d_%H:%M:%S",
    )?)
}

/// Parses date with omitted default suffix
fn date(str: &str) -> Result<NaiveDate, RangeErr> {
    let overlay = overlay(str, "XX/01/01", Align::Leading);
    Ok(NaiveDate::parse_from_str(&overlay?, "%y/%m/%d")?)
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

/// Interprets `NaiveDateTime` in configured timezone.
/// Throws error if time is ambiguous or invalid due to winter/summer time switch
pub fn in_timezone(ndt: &NaiveDateTime) -> Result<DateTime<Tz>, RangeErr> {
    match ndt.and_local_timezone(crate::context::get().config().timezone) {
        LocalResult::Single(single) => Ok(single),
        LocalResult::Ambiguous(_, _) => Err(RangeErr::AmbiguousInTimezone),
        LocalResult::None => Err(RangeErr::InvalidInTimezone),
    }
}

// Calculates first time for a given date.
// Will return the next day, in case date does not exist.
pub fn first_time(nd: &NaiveDate) -> DateTime<Tz> {
    let tz = crate::context::get().config().timezone;
    let ndt = nd.and_time(NaiveTime::default());
    match ndt.and_local_timezone(tz) {
        LocalResult::Single(single) => single,
        LocalResult::Ambiguous(earliest, _) => earliest,
        LocalResult::None => GapInfo::new(&ndt, &tz)
            // True, since we are in `LocalResult::None` case
            .expect("Midnight falls in gap")
            .end
            .expect("Timespans compiled for near future (up to year 2100)"),
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
            Range::AllDay(Span::new(d(2025, 8, 1), d(2025, 9, 1)))
        );
        test(
            "25/01/28-30",
            Range::AllDay(Span::new(d(2025, 1, 28), d(2025, 1, 30)))
        );
        test(
            "25-28",
            Range::AllDay(Span::new(d(2025, 1, 1), d(2028, 1, 1)))
        );
        test(
            "25/08/01_17-18",
            Range::Timed(Span::new(dt(2025, 8, 1, 17, 0, 0), dt(2025, 8, 1, 18, 0, 0)))
        );
        test(
            "25/03/02_15:45-04/01_11:46",
            Range::Timed(Span::new(dt(2025, 3, 2, 15, 45, 00), dt(2025, 4, 1, 11, 46, 00)))
        );

        fn test(str: &str, range: Range) {
            assert_eq!(&range.to_string(), str);
            assert_eq!(Range::from_str(str), Ok(range));
        }

        fn d(y: i32, m: u32, d: u32) -> NaiveDate {
            NaiveDate::from_ymd_opt(y, m, d).unwrap()
        }

        fn dt(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Tz> {
            let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
            let time = NaiveTime::from_hms_opt(h, min, s).unwrap();
            in_timezone(&NaiveDateTime::new(date, time)).unwrap()
        }
    }

    #[test]
    fn range_error() {
        test("25/07/08", RangeErr::MissingEndBound);
        test("25/07/08-07", RangeErr::Empty);
        test("25/09/07_00:30-40", RangeErr::InvalidInTimezone);
        test("25/04/05_23:30-40", RangeErr::AmbiguousInTimezone);

        fn test(str: &str, err: RangeErr) {
            assert_eq!(Range::from_str(str), Err(err));
        }
    }

    #[test]
    fn all_day_range() -> Result<(), RangeErr> {
        let range = Range::from_str("25/09/07-08")?;
        let dt = range.start().dt();
        // This day starts at 01:00
        assert_eq!(dt.time().hour(), 1);
        // And has 23 hours
        assert_eq!(range.time_delta(), TimeDelta::hours(23));
        Ok(())
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
}
