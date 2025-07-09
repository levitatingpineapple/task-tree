use super::range::{Bound, Range, RangeErr};
use chrono::{DateTime, Datelike, Local, Timelike};
use rrule::{Frequency, RRule, RRuleError};
use std::fmt::{self, Display};
use std::{fmt::Formatter, num::ParseIntError, str::FromStr};

#[derive(Debug, PartialEq)]
pub struct Repeat {
    pub rule: RRule,
}

impl Repeat {
    pub fn from_str_in_range(str: &str, range: &Range) -> Result<Repeat, RepeatErr> {
        let mut parts = str.splitn(2, "-"); // components-until
        let mut body_parts = parts.next().expect("first").split("_");
        let mut rule = RRule::new(Frequency::from_str(body_parts.next().expect("first"))?);
        // Decode `%` and `#` components
        while let Some(part) = body_parts.next() {
            if let Some(prefix) = part.strip_prefix('%') {
                rule = rule.interval(prefix.parse::<u16>()?);
            } else if let Some(prefix) = part.strip_prefix("#") {
                rule = rule.count(prefix.parse::<u32>()?)
            }
        }
        // Decode until
        if let Some(until_str) = parts.next() {
            rule = rule.until(Bound::from_str(until_str)?.date_time());
        }
        Ok(Repeat {
            rule: rule.validate(range.start().date_time())?,
        })
    }
}

impl Display for Repeat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rule.get_freq().to_string().to_lowercase())?;
        let interval = self.rule.get_interval();
        if interval != 1 {
            write!(f, "_%{}", interval)?;
        }
        if let Some(count) = self.rule.get_count() {
            write!(f, "_#{}", count)?;
        }
        if let Some(until) = self.rule.get_until() {
            let dt = until.with_timezone(&chrono::Local);
            write!(f, "-{}", dt.format(until_format(&dt)).to_string())?;
        }
        Ok(())
    }
}

#[rustfmt::skip]
fn until_format(dt: &DateTime<Local>) -> &'static str {
    if dt.second() != 0 { return "%y/%m/%d_%H:%M:%S"; }
    if dt.minute() != 0 { return "%y/%m/%d_%H:%M"; }
    if dt.hour()   != 0 { return "%y/%m/%d_%H"; }
    if dt.day()    != 1 { return "%y/%m/%d"; }
    if dt.month()  != 1 { return "%y/%m"; }
                          return "%y";
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum RepeatErr {
    #[error("Invalid frequency: {0}")]
    Frequency(#[from] rrule::ParseError),
    #[error("Not an integer: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("Invalid repeat rule: {0}")]
    Validation(#[from] RRuleError),
    #[error("Range error")]
    Until(#[from] RangeErr),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime};
    use serial_test::serial;

    #[test]
    #[serial] // All timezone tests should be serial
    fn repeat_parsing() -> Result<(), RepeatErr> {
        unsafe {
            std::env::set_var("TZ", "UTC");
        }
        test("daily", "FREQ=DAILY;BYHOUR=4;BYMINUTE=5;BYSECOND=6")?;
        test(
            "weekly",
            "FREQ=WEEKLY;BYHOUR=4;BYMINUTE=5;BYSECOND=6;BYDAY=MO",
        )?;
        test(
            "monthly_%2",
            "FREQ=MONTHLY;INTERVAL=2;BYMONTHDAY=3;BYHOUR=4;BYMINUTE=5;BYSECOND=6",
        )?;
        test(
            "yearly_#10",
            "FREQ=YEARLY;COUNT=10;BYMONTH=2;BYMONTHDAY=3;BYHOUR=4;BYMINUTE=5;BYSECOND=6",
        )?;
        test(
            "weekly_%2-25/08",
            "FREQ=WEEKLY;UNTIL=20250801T000000Z;INTERVAL=2;BYHOUR=4;BYMINUTE=5;BYSECOND=6;BYDAY=MO",
        )?;
        Ok(())
    }

    /// These tests could also break due to implementation details in rrule crate.
    /// In that case update tests, such that  the rule order does not matter
    fn test(str: &str, rrule_str: &str) -> Result<(), RepeatErr> {
        let repeat = Repeat::from_str_in_range(
            str,
            &Range::Timed(dt(25, 02, 03, 04, 05, 06)..dt(25, 02, 03, 07, 08, 09)),
        )?;
        assert_eq!(repeat.to_string(), str);
        assert_eq!(repeat.rule.to_string(), rrule_str);
        Ok(())
    }

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Local> {
        let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
        let time = NaiveTime::from_hms_opt(h, min, s).unwrap();
        crate::session::range::local(&NaiveDateTime::new(date, time)).unwrap()
    }
}
