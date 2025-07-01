use crate::{
    nested::Nested,
    session::{self, Session, SessionErr},
};
use chrono::Local;
use ics::{
    Event,
    properties::{RRule, Sequence, Summary},
};
use markdown::mdast::{List, ListItem, Node};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    str::FromStr,
};

#[derive(Debug, PartialEq, Default)]
pub struct Task {
    done: Option<bool>,
    text: String,
    sessions: Vec<Session>,
    children: Vec<Task>,
}

impl Task {
    pub fn tasks(list: &List) -> Result<Vec<Task>, TaskErr> {
        list.children
            .iter()
            .map(|child| match child {
                Node::ListItem(item) => Task::new(item),
                _ => Err(TaskErr::NotListItem),
            })
            .collect()
    }

    /// Creates task from markdown list item
    /// Any code blocks must be decodable to a session
    pub fn new(list_item: &ListItem) -> Result<Task, TaskErr> {
        let mut child_iter = list_item.children.iter();
        let first_child = child_iter.next().ok_or(TaskErr::EmptyListItem)?;
        let paragraph = if let Node::Paragraph(paragraph) = first_child {
            Ok(paragraph)
        } else {
            Err(TaskErr::MissingParagraph)
        }?;
        let mut task = Task {
            done: list_item.checked,
            ..Default::default()
        };
        // Collect description TODO: Should this be handled by the UTIL?
        for child in &paragraph.children {
            if let Node::InlineCode(inline_code) = child {
                task.sessions
                    .push(Session::from_str(&inline_code.value).map_err(TaskErr::Session)?);
            } else {
                task.text.push_str(&child.to_string());
            }
        }
        // Removes trailing space, present when ther are sessions
        if let Some(pos) = task.text.rfind(|c| c != ' ') {
            task.text.truncate(pos + 1);
        }

        if let Some(second_child) = child_iter.next() {
            if let Node::List(list) = second_child {
                task.children = Task::tasks(list)?
            } else {
                return Err(TaskErr::NotList);
            };
        }

        Ok(task)
    }

    pub fn events(&self) -> Vec<Event> {
        let now = Local::now();
        let dtstamp = session::formatted(now);
        let test: Vec<Event> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let id = self.event_id(i); // TODO: This should take context into account
                let mut event = Event::new(format!("{:x}", id), dtstamp.clone());
                event.push(Summary::new(self.text.clone()));
                // event.push(Description::new(parents.clone()));
                event.push(session.dt_start());
                event.push(session.dt_end());
                // Apple calendar will only update event _once_
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

    // TODO: This still requires context to be created - will have to do dfs on the group tree
    fn event_id(&self, session: usize) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.text.hash(&mut hasher);
        session.hash(&mut hasher);
        hasher.finish()
    }
}

impl Nested for Task {
    fn children(&self) -> &Vec<Self> {
        &self.children
    }
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum TaskErr {
    #[error("Nothing in the list")]
    EmptyListItem,
    #[error("Expected list item")]
    NotListItem,
    #[error("First child should be a paragraph")]
    MissingParagraph,
    #[error("Second child should be a list")]
    NotList,
    #[error("Session error: {0}")]
    Session(SessionErr),
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
    fn task_new() {
        let li = list_item("- [ ] My _special_ task `25/03/28_12:30-14:00` `25/02/03_21:45-22:30`");
        let task = Task {
            done: Some(false),
            text: "My special task".to_string(),
            sessions: vec![
                Session::from_str("25/03/28_12:30-14:00").unwrap(),
                Session::from_str("25/02/03_21:45-22:30").unwrap(),
            ],
            children: vec![],
        };
        assert_eq!(Task::new(&li), Ok(task));
    }
}
