use super::range::{Bound, Range, RangeErr};
use rrule::{Frequency, RRule, RRuleError};
use std::{num::ParseIntError, str::FromStr};

pub fn rule(str: &str, range: &Range) -> Result<RRule, RepeatErr> {
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
    Ok(rule.validate(range.start().date_time())?)
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

    #[test]
    fn daily_rule() -> Result<(), RepeatErr> {
        rule_test("daily", "FREQ=DAILY;BYHOUR=4;BYMINUTE=5;BYSECOND=6")
    }

    #[test]
    fn weekly_rule() -> Result<(), RepeatErr> {
        rule_test(
            "weekly",
            "FREQ=WEEKLY;BYHOUR=4;BYMINUTE=5;BYSECOND=6;BYDAY=MO",
        )
    }

    #[test]
    fn every_second_month() -> Result<(), RepeatErr> {
        rule_test(
            "monthly_%2",
            "FREQ=MONTHLY;INTERVAL=2;BYMONTHDAY=3;BYHOUR=4;BYMINUTE=5;BYSECOND=6",
        )
    }

    #[test]
    fn ten_years() -> Result<(), RepeatErr> {
        rule_test(
            "yearly_#10",
            "FREQ=YEARLY;COUNT=10;BYMONTH=2;BYMONTHDAY=3;BYHOUR=4;BYMINUTE=5;BYSECOND=6",
        )
    }

    #[test]
    fn until_some() -> Result<(), RepeatErr> {
        rule_test(
            "weekly_%2-25/08",
            "FREQ=WEEKLY;UNTIL=20250801T000000Z;INTERVAL=2;BYHOUR=4;BYMINUTE=5;BYSECOND=6;BYDAY=MO",
        )
    }
    /// These tests could also break due to implementation details in rrule crate.
    /// In that case update tests, so the rule order does not matter
    fn rule_test(str: &str, rrule: &str) -> Result<(), RepeatErr> {
        unsafe {
            // Override local timezone to UTC
            std::env::set_var("TZ", "UTC");
        }
        let start = date_time(25, 02, 03, 04, 05, 06);
        let end = date_time(25, 02, 03, 07, 08, 09);
        let repeat_rule = rule(str, &Range::Timed(start..end))?;
        assert_eq!(format!("{}", repeat_rule), rrule);
        Ok(())
    }

    fn date_time(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Local> {
        let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
        let time = NaiveTime::from_hms_opt(h, min, s).unwrap();
        crate::session::range::local(&NaiveDateTime::new(date, time)).unwrap()
    }
}
