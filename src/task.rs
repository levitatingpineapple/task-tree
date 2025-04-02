use crate::session::{Session, SessionError};
use chrono::{DateTime, Local, Utc};
use ics::{
    Event,
    properties::{DtEnd, DtStart, RRule, Sequence, Summary},
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
    text: String,
    sessions: Vec<Session>,
    parent: Option<usize>,
}

impl Task {
    /// Creates task from markdown list item
    pub fn new(item: &ListItem, parent: Option<usize>) -> Result<Task, TaskErr> {
        let node = item.children.get(0).ok_or(TaskErr::EmptyListItem)?;
        let paragraph = if let Node::Paragraph(paragraph) = node {
            Ok(paragraph)
        } else {
            Err(TaskErr::MissingParagraph)
        }?;
        let mut event = Task {
            done: item.checked,
            text: String::new(),
            sessions: Vec::new(),
            parent,
        };
        for child in &paragraph.children {
            if let Node::InlineCode(inline_code) = child {
                event
                    .sessions
                    .push(Session::from_str(&inline_code.value).map_err(TaskErr::Session)?);
            } else {
                event.text.push_str(&child.to_string());
            }
        }
        if let Some(pos) = event.text.rfind(|c| c != ' ') {
            event.text.truncate(pos + 1);
        }
        Ok(event)
    }

    pub fn events(&self, tasks: &Vec<Task>) -> Vec<Event> {
        let now = Local::now();
        let zulu_now = zulu(now);
        let summary = self.summary(tasks);
        let test: Vec<Event> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let id = Task::event_id(&summary, i);
                let mut event = Event::new(format!("{:x}", id), zulu_now.clone());
                event.push(Summary::new(summary.clone()));
                event.push(DtStart::new(zulu(session.start)));
                event.push(DtEnd::new(zulu(session.end)));
                // Apple calendar will only update even _once_
                // even when `DTSTAMP` and `LAST-MODIFIED` are incremented
                // setting sequence number to current unix timestamp
                // allows updating events wihout retaining any state
                event.push(Sequence::new(now.timestamp().to_string()));
                if let Some(rrule) = &session.rrule {
                    event.push(RRule::new(rrule.to_string()));
                }
                event
            })
            .collect();

        return test;
    }

    // TODO: Consider that the additional items could go into description
    fn summary(&self, tasks: &Vec<Task>) -> String {
        let mut summaries = Vec::new();
        summaries.push(self.text.clone());
        let mut parent_idx = self.parent;
        while let Some(idx) = parent_idx {
            if let Some(parent_task) = tasks.get(idx) {
                summaries.push(parent_task.text.clone());
                parent_idx = parent_task.parent;
            } else {
                break;
            }
        }
        summaries.join(". ")
    }

    /// Each session is uniquely identified by hash of
    /// the task summary and it's index
    fn event_id(full_summary: &str, session: usize) -> u64 {
        let mut hasher = DefaultHasher::new();
        full_summary.hash(&mut hasher);
        session.hash(&mut hasher);
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
            text: "My special task".to_string(),
            sessions: vec![
                Session::from_str("25/03/28_12:30-14:00").unwrap(),
                Session::from_str("25/02/03_21:45-22:30").unwrap(),
            ],
            parent: None,
        };
        assert_eq!(Task::new(&li, None), Ok(task));
    }
}
