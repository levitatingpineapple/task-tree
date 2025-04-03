mod range;
mod repeat;
mod timestamp;

use std::str::FromStr;

use chrono::{DateTime, Utc};
use ics::{
    parameters,
    properties::{DtEnd, DtStart},
};
use range::{Range, RangeErr};
use repeat::{Repeat, RepeatErr};
use rrule::RRule;

#[derive(Debug, PartialEq)]
pub struct Session {
    pub range: Range,
    pub rrule: Option<RRule>,
}

#[derive(Debug, PartialEq)]
pub enum SessionErr {
    Range(RangeErr),
    Repeat(RepeatErr),
}

impl Session {
    pub fn dt_start<'a>(&self) -> DtStart<'a> {
        match self.range.start {
            timestamp::Ts::Date(nd) => {
                let mut dt = DtStart::new(nd.format("%Y%m%d").to_string());
                dt.append(parameters!("VALUE" => "DATE"));
                dt
            }
            timestamp::Ts::Timed(dt) => DtStart::new(formatted(dt.clone())),
        }
    }

    pub fn dt_end<'a>(&self) -> DtEnd<'a> {
        match self.range.end {
            timestamp::Ts::Date(nd) => {
                let mut dt = DtEnd::new(nd.format("%Y%m%d").to_string());
                dt.append(parameters!("VALUE" => "DATE"));
                dt
            }
            timestamp::Ts::Timed(dt) => DtEnd::new(formatted(dt.clone())),
        }
    }
}

impl FromStr for Session {
    type Err = SessionErr;

    fn from_str(str: &str) -> Result<Session, SessionErr> {
        let mut parts = str.splitn(2, "|");
        let range = Range::from_str(parts.next().expect("first")).map_err(SessionErr::Range)?;
        let rrule = parts
            .next()
            .map(|s| Repeat::from_str(s).map_err(SessionErr::Repeat))
            .transpose()?
            .map(|r| r.validated_in(&range.start).map_err(SessionErr::Repeat))
            .transpose()?;
        Ok(Session { range, rrule })
    }
}

/// Formats UTC DateTime to ICS ZULU format - trailing Z
pub fn formatted(dt: DateTime<Utc>) -> String {
    dt.format("%Y%m%dT%H%M%SZ").to_string()
}
