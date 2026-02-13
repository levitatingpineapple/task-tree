use super::range::{Bound, Range, RangeErr};
use chrono::{DateTime, Datelike, Timelike};
use chrono_tz::Tz;
use rrule::{Frequency, NWeekday, RRule, RRuleError, Weekday};
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
        let first_part = body_parts.next().expect("first");
        let mut rule = match Frequency::from_str(first_part) {
            Ok(frequency) => RRule::new(frequency),
            Err(_) => first_part
                .split(',')
                .map(|s| {
                    from_str(s)
                        .map(|wd| NWeekday::new(None, wd))
                        .ok_or(RepeatErr::Frequency)
                })
                .collect::<Result<Vec<NWeekday>, RepeatErr>>()
                .and_then(|nwd| {
                    if nwd.is_empty() {
                        Err(RepeatErr::Frequency)
                    } else {
                        Ok(RRule::new(Frequency::Weekly).by_weekday(nwd))
                    }
                })?,
        };
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
        if self.rule.get_by_weekday().len() > 1 {
            // TODO: This will not correctly format all Rrules
            // repeat should be it's own type with delayed validation
            let mut first = true;
            for weekday in self.rule.get_by_weekday() {
                let wd = format!("{}", weekday);
                if first {
                    first = false;
                } else {
                    write!(f, ",")?;
                }
                write!(f, "{}", wd.to_lowercase())?;
            }
        } else {
            write!(f, "{}", self.rule.get_freq().to_string().to_lowercase())?;
        }

        let interval = self.rule.get_interval();
        if interval != 1 {
            write!(f, "_%{}", interval)?;
        }
        if let Some(count) = self.rule.get_count() {
            write!(f, "_#{}", count)?;
        }
        if let Some(until) = self.rule.get_until() {
            let tz = crate::context().config().timezone;
            let dt = until.with_timezone(&tz);
            write!(f, "-{}", dt.format(until_format(&dt)).to_string())?;
        }
        Ok(())
    }
}

#[rustfmt::skip]
fn until_format(dt: &DateTime<Tz>) -> &'static str {
    if dt.second() != 0 { return "%y/%m/%d_%H:%M:%S"; }
    if dt.minute() != 0 { return "%y/%m/%d_%H:%M"; }
    if dt.hour()   != 0 { return "%y/%m/%d_%H"; }
    if dt.day()    != 1 { return "%y/%m/%d"; }
    if dt.month()  != 1 { return "%y/%m"; }
                          return "%y";
}

fn from_str(str: &str) -> Option<Weekday> {
    match str {
        "mo" => Some(Weekday::Mon),
        "tu" => Some(Weekday::Tue),
        "we" => Some(Weekday::Wed),
        "th" => Some(Weekday::Thu),
        "fr" => Some(Weekday::Fri),
        "sa" => Some(Weekday::Sat),
        "su" => Some(Weekday::Sun),
        _ => None,
    }
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum RepeatErr {
    #[error("Invalid frequency")]
    Frequency,
    #[error("Not an integer: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("Invalid repeat rule: {0}")]
    Validation(#[from] RRuleError),
    #[error("Range error")]
    Until(#[from] RangeErr),
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
//     use serial_test::serial;

//     #[test]
//     #[serial] // All timezone tests should be serial
//     fn repeat_parsing() -> Result<(), RepeatErr> {
//         unsafe {
//             std::env::set_var("TZ", "UTC");
//         }
//         test("daily", "FREQ=DAILY;BYHOUR=4;BYMINUTE=5;BYSECOND=6")?;
//         test(
//             "weekly",
//             "FREQ=WEEKLY;BYHOUR=4;BYMINUTE=5;BYSECOND=6;BYDAY=MO",
//         )?;
//         test(
//             "monthly_%2",
//             "FREQ=MONTHLY;INTERVAL=2;BYMONTHDAY=3;BYHOUR=4;BYMINUTE=5;BYSECOND=6",
//         )?;
//         test(
//             "yearly_#10",
//             "FREQ=YEARLY;COUNT=10;BYMONTH=2;BYMONTHDAY=3;BYHOUR=4;BYMINUTE=5;BYSECOND=6",
//         )?;
//         test(
//             "weekly_%2-25/08",
//             "FREQ=WEEKLY;UNTIL=20250801T000000Z;INTERVAL=2;BYHOUR=4;BYMINUTE=5;BYSECOND=6;BYDAY=MO",
//         )?;
//         test(
//             "mo,we,su_#5",
//             "FREQ=WEEKLY;COUNT=5;BYHOUR=4;BYMINUTE=5;BYSECOND=6;BYDAY=MO,WE,SU",
//         )?;
//         Ok(())
//     }

//     /// These tests could also break due to implementation details in rrule crate.
//     /// In that case update tests, such that  the rule order does not matter
//     fn test(str: &str, rrule_str: &str) -> Result<(), RepeatErr> {
//         let repeat = Repeat::from_str_in_range(
//             str,
//             &Range::Timed(dt(25, 02, 03, 04, 05, 06)..dt(25, 02, 03, 07, 08, 09)),
//         )?;
//         assert_eq!(repeat.to_string(), str);
//         assert_eq!(repeat.rule.to_string(), rrule_str);
//         Ok(())
//     }

//     fn dt(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Tz> {
//         let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
//         let time = NaiveTime::from_hms_opt(h, min, s).unwrap();
//         crate::session::range::in_timezone(&NaiveDateTime::new(date, time)).unwrap()
//     }
// }
