use std::{any::Any, str::FromStr};

use markdown::mdast::{ListItem, Node};

use crate::session::{Session, SessionError};

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub struct Event {
    done: Option<bool>,
    summary: String,
    sessions: Vec<Session>,
}

impl Event {
    fn new(item: &ListItem) -> Result<Event, EventErr> {
        let node = item.children.get(0).ok_or(EventErr::EmptyListItem)?;
        let paragraph = if let Node::Paragraph(paragraph) = node {
            Ok(paragraph)
        } else {
            Err(EventErr::MissingParagraph)
        }?;
        let mut event = Event {
            done: item.checked,
            summary: String::new(),
            sessions: Vec::new(),
        };
        for child in &paragraph.children {
            if let Node::InlineCode(inline_code) = child {
                event
                    .sessions
                    .push(Session::from_str(&inline_code.value).map_err(EventErr::Session)?);
            } else {
                event.summary.push_str(&child.to_string());
            }
        }
        // Trim trailing spaces
        if let Some(pos) = event.summary.rfind(|c| c != ' ') {
            event.summary.truncate(pos + 1);
        }
        Ok(event)
    }
}

#[derive(Debug, PartialEq)]
enum EventErr {
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
        let event = Event {
            done: Some(false),
            summary: "My special task".to_string(),
            sessions: vec![
                Session::from_str("25/03/28_12:30-14:00").unwrap(),
                Session::from_str("25/02/03_21:45-22:30").unwrap(),
            ],
        };
        assert_eq!(Event::new(&li), Ok(event));
    }
}
