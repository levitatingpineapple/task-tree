mod range;
mod repeat;
mod timestamp;

use std::str::FromStr;

use range::{Range, RangeErr};
use repeat::{Repeat, RepeatErr};
use timestamp::Ts;

#[derive(Debug, PartialEq)]
pub struct Session {
    range: Range,
    repeat: Option<Repeat>,
}

#[derive(Debug, PartialEq)]
pub enum SessionErr {
    Range(RangeErr),
    Repeat(RepeatErr),
}

impl Session {
    fn start(&self) -> String {
        todo!()
    }
}

impl FromStr for Session {
    type Err = SessionErr;

    fn from_str(str: &str) -> Result<Session, SessionErr> {
        let mut parts = str.splitn(2, "|");
        let range = Range::from_str(parts.next().expect("first")).map_err(SessionErr::Range)?;
        let repeat = parts
            .next()
            .map(|s| Repeat::from_str(s).map_err(SessionErr::Repeat))
            .transpose()?;
        todo!()
    }
}
