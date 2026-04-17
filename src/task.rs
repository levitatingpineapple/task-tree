use crate::{
    ranged_err::{Ranged, ranged},
    session::{Session, SessionErr, range::Span},
    tasktree::TotalTime,
    tree::{Child, Parent},
};
use chrono::{DateTime, TimeDelta};
use chrono_tz::Tz;
use markdown::mdast::{List, ListItem, Node};
use mdast_util_to_markdown::{Options, to_markdown_with_options};
use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

#[derive(Debug, Default, PartialEq)]
pub struct Task {
    pub done: Option<bool>,
    pub text: String,
    pub sessions: Vec<Session>,
    pub sub_tasks: Vec<Task>,
}

impl Task {
    /// Creates task from markdown list item
    /// Any code blocks expected to be decodable to a `Session`
    fn new(list_item: &ListItem) -> Result<Task, Ranged<TaskErr>> {
        let mut child_iter = list_item.children.iter();
        let first_child = child_iter
            .next()
            .ok_or(ranged(TaskErr::EmptyListItem, list_item.position.as_ref()))?;
        let paragraph = if let Node::Paragraph(paragraph) = first_child {
            Ok(paragraph)
        } else {
            Err(ranged(
                TaskErr::MissingParagraph,
                list_item.position.as_ref(),
            ))
        }?;
        let mut task = Task {
            done: list_item.checked,
            ..Default::default()
        };
        // Populate text and sessions
        for child in &paragraph.children {
            if let Node::InlineCode(inline_code) = child {
                task.sessions.push(
                    Session::from_str(&inline_code.value)
                        .map_err(|e| ranged(TaskErr::Session(e), inline_code.position.as_ref()))?,
                );
            } else {
                // On new-line the space is missing
                if task.text.chars().last() != Some(' ') {
                    task.text.push(' ');
                }
                task.text.push_str(&to_md_string(&child));
            }
        }
        task.text = task.text.trim().to_string();
        // Populate child tasks
        if let Some(second_child) = child_iter.next() {
            if let Node::List(list) = second_child {
                task.sub_tasks = Task::new_tasks(list, task.done)?
            } else {
                return Err(ranged(TaskErr::NotList, second_child.position()));
            };
        }
        Ok(task)
    }

    /// Given a markdown list - returns a vector or tasks
    pub fn new_tasks(list: &List, parent_done: Option<bool>) -> Result<Vec<Task>, Ranged<TaskErr>> {
        list.children
            .iter()
            .map(|child| match child {
                Node::ListItem(list_item) => {
                    if parent_done == Some(true) && list_item.checked != Some(true) {
                        Err(ranged(
                            TaskErr::ParentCompleted,
                            list_item.position.as_ref(),
                        ))
                    } else {
                        Task::new(list_item)
                    }
                }
                _ => Err(ranged(TaskErr::NotListItem, child.position())),
            })
            .collect()
    }

    /// Recursive `Display` helper
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
        for child in &self.sub_tasks {
            child.fmt_recursive(f, level + 1)?;
        }
        Ok(())
    }
}

impl TotalTime for Task {
    fn time_delta(&self, span: Span<DateTime<Tz>>) -> TimeDelta {
        let sub_tasks = self
            .sub_tasks
            .iter()
            .fold(TimeDelta::zero(), |time, sub_task| {
                time + sub_task.time_delta(span)
            });
        let sessions = self
            .sessions
            .iter()
            .fold(TimeDelta::zero(), |time, session| {
                time + session.time_delta(span)
            });
        sessions + sub_tasks
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt_recursive(f, 0)
    }
}

impl Parent<Task> for Task {
    fn children(&self) -> &Vec<Task> {
        &self.sub_tasks
    }

    fn children_mut(&mut self) -> &mut Vec<Task> {
        &mut self.sub_tasks
    }

    fn into_children(self) -> Vec<Task> {
        self.sub_tasks
    }

    fn move_data_from(&mut self, other: &mut Task) {
        assert!(self.text == other.text); // Sanity check
        self.done = other.done;
        self.sessions.extend(other.sessions.drain(..));
    }
}

impl Child for Task {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.text.clone()
    }

    fn new(id: Self::Id) -> Self {
        Self {
            text: id,
            ..Default::default()
        }
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
    #[error("Incomplete task for completed parent")]
    ParentCompleted,
    #[error("Session error: {0}")]
    Session(#[from] SessionErr),
}

fn to_md_string(child: &Node) -> String {
    to_markdown_with_options(
        &child,
        &Options {
            emphasis: '_',
            ..Default::default()
        },
    )
    .expect("Valid node types")
    .trim_end_matches("\n")
    .into()
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
        // Keeps markdown formatting and corrects emphasis
        let li = list_item(
            "- [ ] My *special* [Link](https://example.com) task `25/03/28_12:30-14:00` `25/02/03_21:45-22:30`",
        );
        let task = Task {
            done: Some(false),
            text: "My _special_ [Link](https://example.com) task".to_string(),
            sessions: vec![
                Session::from_str("25/03/28_12:30-14:00").unwrap(),
                Session::from_str("25/02/03_21:45-22:30").unwrap(),
            ],
            sub_tasks: vec![],
        };
        assert_eq!(Task::new(&li), Ok(task));
    }
}
