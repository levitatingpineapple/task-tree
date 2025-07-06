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

// TODO: Cover Repeat with tests
