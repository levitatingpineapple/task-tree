use super::timestamp::{Ts, TsErr};
use rrule::{Frequency, RRule, RRuleError, Unvalidated};
use std::{num::ParseIntError, str::FromStr};

#[derive(Debug, PartialEq)]
pub struct Repeat {
    rule: RRule<Unvalidated>,
}

impl Repeat {
    /// Validates the repeat rule for given start timestamp
    pub fn validated_in(self, start: &Ts) -> Result<RRule, RepeatErr> {
        let date_time = start.as_utc()?;
        let validated = self.rule.validate(date_time)?;
        Ok(validated)
    }
}

impl FromStr for Repeat {
    type Err = RepeatErr;

    fn from_str(str: &str) -> Result<Repeat, RepeatErr> {
        let mut parts = str.splitn(2, "-");
        let mut body_parts = parts.next().expect("first").split("_");
        let ts = parts
            .next()
            .map(|str| Ts::from_str(str))
            .transpose()
            .map_err(RepeatErr::Ts)?;
        let mut rule = RRule::new(Frequency::from_str(body_parts.next().expect("first"))?);
        if let Some(ts) = ts {
            rule = rule.until(ts.as_utc().map_err(RepeatErr::Ts)?);
        }
        while let Some(part) = parts.next() {
            if let Some(prefix) = part.strip_prefix('%') {
                rule = rule.interval(prefix.parse::<u16>()?);
            } else if let Some(prefix) = part.strip_prefix("#") {
                rule = rule.count(prefix.parse::<u32>()?)
            }
        }
        Ok(Repeat { rule })
    }
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum RepeatErr {
    #[error("Invalid frequency: {0}")]
    Frequency(#[from] rrule::ParseError),
    #[error("Not an integer: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("Invalid timestamp: {0}")]
    Ts(#[from] TsErr),
    #[error("Invalid repeat rule: {0}")]
    Validation(#[from] RRuleError),
}

// TODO: Cover Repeat with tests
