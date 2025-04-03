use super::{
    range::Range,
    timestamp::{Ts, TsErr},
};
use rrule::{Frequency, RRule, RRuleError, Tz, Unvalidated};
use std::{num::ParseIntError, str::FromStr};

#[derive(Debug, PartialEq)]
pub struct Repeat {
    rule: RRule<Unvalidated>,
}

#[derive(Debug, PartialEq)]
pub enum RepeatErr {
    Frequency(rrule::ParseError),
    Interval(ParseIntError),
    Count(ParseIntError),
    Ts(TsErr),
    Validation(RRuleError),
}

impl Repeat {
    /// Validates the repeat rule for given start timestamp
    pub fn validated_in(self, range: &Range) -> Result<RRule, RepeatErr> {
        let start = match range {
            Range::AllDay(range) => Ts::Date(range.start),
            Range::Timed(range) => Ts::DateTime(range.start),
        };
        self.rule
            .validate(start.as_utc().map_err(RepeatErr::Ts)?)
            .map_err(RepeatErr::Validation)
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
        let mut rule = RRule::new(
            Frequency::from_str(body_parts.next().expect("first")).map_err(RepeatErr::Frequency)?,
        );
        if let Some(ts) = ts {
            rule = rule.until(ts.as_utc().map_err(RepeatErr::Ts)?);
        }
        while let Some(part) = parts.next() {
            if let Some(prefix) = part.strip_prefix('%') {
                rule = rule.interval(prefix.parse::<u16>().map_err(RepeatErr::Interval)?);
            } else if let Some(prefix) = part.strip_prefix("#") {
                rule = rule.count(prefix.parse::<u32>().map_err(RepeatErr::Count)?)
            }
        }
        Ok(Repeat { rule })
    }
}

// TODO: Cover Repeat with tests
