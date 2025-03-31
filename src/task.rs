use crate::session::{Session, SessionError};
use chrono::{DateTime, Local, Utc};
use ics::{
    Event,
    properties::{DtEnd, DtStart, RRule, Summary},
};
use markdown::mdast::{ListItem, Node};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    str::FromStr,
};

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub struct Task {
    done: Option<bool>,
    summary: String,
    sessions: Vec<Session>,
}

impl Task {
    /// Creates task from markdown list item
    pub fn new(item: &ListItem) -> Result<Task, TaskErr> {
        let node = item.children.get(0).ok_or(TaskErr::EmptyListItem)?;
        let paragraph = if let Node::Paragraph(paragraph) = node {
            Ok(paragraph)
        } else {
            Err(TaskErr::MissingParagraph)
        }?;
        let mut event = Task {
            done: item.checked,
            summary: String::new(),
            sessions: Vec::new(),
        };
        for child in &paragraph.children {
            if let Node::InlineCode(inline_code) = child {
                event
                    .sessions
                    .push(Session::from_str(&inline_code.value).map_err(TaskErr::Session)?);
            } else {
                event.summary.push_str(&child.to_string());
            }
        }
        if let Some(pos) = event.summary.rfind(|c| c != ' ') {
            event.summary.truncate(pos + 1);
        }
        Ok(event)
    }

    pub fn events(&self) -> Vec<Event> {
        let now = zulu(Local::now());
        self.sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let id = self.event_id(i);
                let mut event = Event::new(format!("{:x}", id), now.clone());
                event.push(Summary::new(&self.summary));
                event.push(DtStart::new(zulu(session.start)));
                event.push(DtEnd::new(zulu(session.end)));
                if let Some(rrule) = &session.rrule {
                    event.push(RRule::new(rrule.to_string()));
                }
                event
            })
            .collect()
    }

    /// Each session is uniquely identified by hash of
    /// the task summary and it's index
    fn event_id(&self, session_index: usize) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.summary.hash(&mut hasher);
        session_index.hash(&mut hasher);
        hasher.finish()
    }
}

/// Formats local time to ICS zulu time UTC+0
fn zulu(local: DateTime<Local>) -> String {
    local
        .with_timezone(&Utc)
        .format("%Y%m%dT%H%M%SZ")
        .to_string()
}

#[derive(Debug, PartialEq)]
pub enum TaskErr {
    EmptyListItem,
    MissingParagraph,
    Session(SessionError),
}

#[cfg(test)]
mod tests {

    use markdown::{ParseOptions, to_mdast};

    use super::*;

    fn list_item(str: &str) -> ListItem {
        if let Node::ListItem(item) = to_mdast(str, &ParseOptions::gfm())
            .unwrap()
            .children()
            .unwrap()
            .get(0)
            .unwrap()
            .children()
            .unwrap()
            .get(0)
            .unwrap()
        {
            item.to_owned()
        } else {
            unreachable!("Expect valid list item in markdown");
        }
    }

    #[test]
    fn event_new() {
        let li = list_item("- [ ] My _special_ task `25/03/28_12:30-14:00` `25/02/03_21:45-22:30`");
        let task = Task {
            done: Some(false),
            summary: "My special task".to_string(),
            sessions: vec![
                Session::from_str("25/03/28_12:30-14:00").unwrap(),
                Session::from_str("25/02/03_21:45-22:30").unwrap(),
            ],
        };
        assert_eq!(Task::new(&li), Ok(task));
    }

    #[test]
    fn foo() {
        let li = list_item(
            "- [ ] My _special_ task `25/03/28_12:30-14:00` `25/02/03_18:45-21:30|daily_#5`",
        );
        let task = Task::new(&li).unwrap();
        for event in task.events() {
            println!("{}", event);
        }
    }
}
