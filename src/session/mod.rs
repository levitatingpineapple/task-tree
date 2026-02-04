mod range;
mod repeat;

use chrono::{DateTime, Duration, TimeZone, Timelike, Utc};
use ics::{
    parameters,
    properties::{DtEnd, DtStart},
};
use range::{Range, RangeErr};
use repeat::{Repeat, RepeatErr};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, PartialEq)]
pub struct Session {
    pub range: Range,
    pub repeat: Option<Repeat>,
}

impl Session {
    pub fn next_hour(tz: chrono_tz::Tz, offset: i64) -> Session {
        let now = chrono::Utc::now().with_timezone(&tz);
        let start = (now + Duration::hours(offset + 1))
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();
        let end = start + Duration::hours(1);
        Session {
            range: Range::Timed(start..end),
            repeat: None,
        }
    }

    pub fn dt_start<'a>(&self) -> DtStart<'a> {
        match &self.range {
            Range::AllDay(r) => {
                let mut dt = DtStart::new(r.start.format("%Y%m%d").to_string());
                dt.append(parameters!("VALUE" => "DATE"));
                dt
            }
            Range::Timed(r) => DtStart::new(ics_format(&r.start)),
        }
    }

    pub fn dt_end<'a>(&self) -> DtEnd<'a> {
        match &self.range {
            Range::AllDay(r) => {
                let mut dt = DtEnd::new(r.end.format("%Y%m%d").to_string());
                dt.append(parameters!("VALUE" => "DATE"));
                dt
            }
            Range::Timed(r) => DtEnd::new(ics_format(&r.end)),
        }
    }
}

impl FromStr for Session {
    type Err = SessionErr;

    fn from_str(str: &str) -> Result<Session, SessionErr> {
        let mut parts = str.splitn(2, "|");
        let range = Range::from_str(parts.next().expect("first"))?;
        let rrule = parts
            .next()
            .map(|s| Repeat::from_str_in_range(s, &range))
            .transpose()?;
        Ok(Session {
            range,
            repeat: rrule,
        })
    }
}

impl Display for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.range)?;
        if let Some(repeat) = &self.repeat {
            write!(f, "|{}", repeat)?;
        }
        Ok(())
    }
}

/// Normalizes time to UTC-0 (ZULU) and formats for ICS with trailing Z
pub fn ics_format<T: TimeZone>(dt: &DateTime<T>) -> String {
    dt.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ").to_string()
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SessionErr {
    #[error("Range: {0}")]
    Range(#[from] RangeErr),
    #[error("Repeat: {0}")]
    Repeat(#[from] RepeatErr),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_parsing() -> Result<(), SessionErr> {
        let str = "25/08/22-25|monthly";
        let session = Session::from_str(str)?;
        assert_eq!(str, session.to_string());
        Ok(())
    }
}
