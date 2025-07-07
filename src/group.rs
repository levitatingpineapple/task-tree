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
                        let tasks = Task::tasks(list)?;
                        root.last_added().tasks = tasks;
                    }
                    _ => { /* Ignore other node types */ }
                }
            }
        }
        Ok(root)
    }

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
