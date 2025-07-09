use std::fmt::{self, Display, Formatter};

use crate::{
    nested::NestedIter,
    task::{Task, TaskErr},
};
use markdown::mdast::Node;

#[derive(Debug, Default, PartialEq)]
pub struct Group {
    pub text: String,
    pub children: Vec<Group>,
    pub tasks: Vec<Task>,
}

impl Group {
    /// Creates root group from markdown abstract syntax tree
    pub fn from_mdast(mdast: Node) -> Result<Self, GroupErr> {
        let mut root = Group::new("Root");

        if let Some(children) = mdast.children() {
            for child in children {
                match child {
                    Node::Heading(heading) => {
                        let foo = root
                            .last_sub(heading.depth - 1)
                            .ok_or(GroupErr::HeadingOrder)?;
                        foo.children.push(Group::new(child.to_string()));
                    }
                    Node::List(list) => {
                        let tasks = Task::new_tasks(list)?;
                        root.last_added().tasks = tasks;
                    }
                    _ => { /* Ignore other node types */ }
                }
            }
        }
        Ok(root)
    }

    /// Creates an empty group with a name
    pub fn new<S: Into<String>>(text: S) -> Self {
        Group {
            text: text.into(),
            ..Default::default()
        }
    }

    /// Returns last subcategory at some level
    fn last_sub(&mut self, depth: u8) -> Option<&mut Group> {
        if depth == 0 {
            Some(self)
        } else {
            self.children.last_mut().map(|s| s.last_sub(depth - 1))?
        }
    }

    /// Assuming tree is built depth-first returns last added element
    fn last_added(&mut self) -> &mut Group {
        if self.children.is_empty() {
            self
        } else {
            // Unwrap required due to borrow checker..
            self.children.last_mut().unwrap().last_added()
        }
    }

    fn fmt_recursive(&self, f: &mut Formatter<'_>, level: usize) -> fmt::Result {
        if level > 0 {
            write!(f, "{} {}\n\n", "#".repeat(level), self.text)?;
        }
        for task in &self.tasks {
            write!(f, "{}", task)?;
        }
        write!(f, "\n")?;
        for child in &self.children {
            child.fmt_recursive(f, level + 1)?;
        }
        Ok(())
    }
}

impl Display for Group {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Headings start at level 1 with 0 being file itself
        self.fmt_recursive(f, 0)
    }
}

impl NestedIter for Group {
    fn children(&self) -> &Vec<Self> {
        &self.children
    }
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum GroupErr {
    #[error("Heading parent missing")]
    HeadingOrder,
    #[error("Task error: {0}")]
    Ts(#[from] TaskErr),
}

// TODO: Add heading tests
