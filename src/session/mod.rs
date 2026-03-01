#[allow(dead_code)]
mod range;
mod repeat;

use chrono::{DateTime, Duration, TimeDelta, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use ics::{
    parameters,
    properties::{DtEnd, DtStart},
};
use range::{Range, RangeErr};
use repeat::{Repeat, RepeatErr};
use std::{fmt::Display, ops::Add, str::FromStr};

use crate::{session::repeat::rrule_tz, tasktree::TotalTime};

pub use range::{Span, first_time};

#[derive(Debug, PartialEq)]
pub struct Session {
    pub range: Range,
    pub repeat: Option<Repeat>,
}

impl Session {
    pub fn next_hour(tz: Tz, offset: i64) -> Session {
        let now = chrono::Utc::now().with_timezone(&tz);
        let start = (now + Duration::hours(offset + 1))
            .with_minute(0)
            .expect("valid minute")
            .with_second(0)
            .expect("valid second");
        let end = start + Duration::hours(1);
        Session {
            range: Range::Timed(Span::new(start, end)),
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

impl TotalTime for Session {
    /// Calculates total time a session is active in some time range
    fn time_delta(&self, span: Span<DateTime<Tz>>) -> TimeDelta {
        let time_delta = self.range.time_delta();
        let repeats = if let Some(repeat) = &self.repeat {
            // It should be fine to call unchecked, since bounds are added
            // set includes the initial session
            rrule::RRuleSet::new(rrule_tz(self.range.start().dt()))
                .rrule(repeat.rule.clone())
                .after(rrule_tz(span.start - time_delta))
                .before(rrule_tz(span.end))
                // NOTE: `rrule` lib will skip repeats, which can't be resolved in a given timezone
                .all_unchecked()
        } else {
            vec![rrule_tz(self.range.start().dt())]
        };
        repeats
            .into_iter()
            .map(|repeat| {
                let start = rrule_tz(span.start).max(repeat);
                let end = rrule_tz(span.end).min(repeat + time_delta);
                (end - start).max(TimeDelta::zero())
            })
            .fold(TimeDelta::zero(), TimeDelta::add)
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

    #[test]
    fn time_delta() -> Result<(), SessionErr> {
        // Session in range
        test("25/10/10_13-14", "25/10/10-11", TimeDelta::hours(1))?;
        // Range in session
        test("25/10/10-11", "25/10/10_13-14", TimeDelta::hours(1))?;
        // Session during range start
        test("25/10/10-11", "25/10/09_22-10_01", TimeDelta::hours(1))?;
        // Session during range end
        test("25/10/10-11", "25/10/10_23-11_02", TimeDelta::hours(1))?;
        // Repeated session
        test("25/10/10_14-15|daily", "25/11/10-14", TimeDelta::hours(4))?;
        // Repeated session on range boundary
        test(
            "25/10/10_23:15-11_00:15|daily",
            "25/11/10-14",
            TimeDelta::hours(4),
        )?;

        fn test(session: &str, range: &str, time_delta: TimeDelta) -> Result<(), SessionErr> {
            let range = Range::from_str(range)?;
            let span = Span::new(range.start().dt(), range.end().dt());
            println!("{:?}", span);
            assert_eq!(Session::from_str(session)?.time_delta(span), time_delta);
            Ok(())
        }

        Ok(())
    }
}
