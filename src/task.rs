use crate::{
    nested::NestedIter,
    session::{Session, SessionErr},
};
use markdown::mdast::{List, ListItem, Node};
use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

#[derive(Debug, Default, PartialEq)]
pub struct Task {
    pub done: Option<bool>,
    pub text: String,
    pub sessions: Vec<Session>,
    pub children: Vec<Task>,
}

impl Task {
    /// Creates task from markdown list item
    /// Any code blocks expected to be decodable to a `Session`
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
        // Populate text and sessions
        for child in &paragraph.children {
            if let Node::InlineCode(inline_code) = child {
                task.sessions
                    .push(Session::from_str(&inline_code.value).map_err(TaskErr::Session)?);
            } else {
                task.text.push_str(&child.to_string());
            }
        }
        // TODO: Handle long task text line-breaks
        task.text = task.text.trim().to_string();
        // Populate child tasks
        if let Some(second_child) = child_iter.next() {
            if let Node::List(list) = second_child {
                task.children = Task::new_tasks(list)?
            } else {
                return Err(TaskErr::NotList);
            };
        }
        Ok(task)
    }

    /// Given a markdown list - returns a vector or tasks
    pub fn new_tasks(list: &List) -> Result<Vec<Task>, TaskErr> {
        list.children
            .iter()
            .map(|child| match child {
                Node::ListItem(item) => Task::new(item),
                _ => Err(TaskErr::NotListItem),
            })
            .collect()
    }

    fn fmt_recursive(&self, f: &mut Formatter<'_>, level: usize) -> fmt::Result {
        write!(f, "{}-", "  ".repeat(level))?;
        if let Some(done) = self.done {
            write!(f, " [{}]", if done { 'x' } else { ' ' })?;
        }
        write!(f, " {}", self.text)?;
        for session in &self.sessions {
            write!(f, " `{}`", session)?;
        }
        write!(f, "\n")?;
        for child in &self.children {
            child.fmt_recursive(f, level + 1)?;
        }
        Ok(())
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt_recursive(f, 0)
    }
}

impl NestedIter for Task {
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
