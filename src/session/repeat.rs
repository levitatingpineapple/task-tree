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
            let ut = rrule_utc(Bound::from_str(until_str)?.dt());
            dbg!(ut);
            rule = rule.until(ut);
        }
        // TODO: Also validate that repeat interval is larger than `range.time_delta` which the library does not do...
        Ok(Repeat {
            rule: rule.validate(rrule_utc(range.start().dt()))?,
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
            let tz = crate::context::get().config().timezone;
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

pub fn rrule_utc(dt: DateTime<Tz>) -> DateTime<rrule::Tz> {
    dt.with_timezone(&rrule::Tz::UTC)
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
