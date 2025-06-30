#![allow(dead_code)]
use crate::task::{Task, TaskErr};
use markdown::mdast::Node;

#[derive(Debug, PartialEq, Default)]
pub struct Group {
    text: String,
    sub: Vec<Group>,
    tasks: Vec<Task>,
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
                        foo.sub.push(Group::new(child.to_string()));
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
    pub fn last_sub(&mut self, depth: u8) -> Option<&mut Group> {
        if depth == 0 {
            Some(self)
        } else {
            self.sub.last_mut().map(|s| s.last_sub(depth - 1))?
        }
    }

    /// Assuming tree is built depth-first returns last added element
    pub fn last_added(&mut self) -> &mut Group {
        if self.sub.is_empty() {
            self
        } else {
            // Unwrap required due to borrow checker..
            self.sub.last_mut().unwrap().last_added()
        }
    }
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum GroupErr {
    #[error("Heading parent missing")]
    HeadingOrder,
    #[error("Task error: {0}")]
    Ts(#[from] TaskErr),
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use markdown::{ParseOptions, to_mdast};

    use super::*;

    #[test]
    fn new_tree() {
        let markdown = indoc! {r#"
            # One
            - [ ] Task1
            - [x] Task2
            ## SubA
            ### Nested
            ## SubB
            - [ ] WORK
            # Two
        "#};

        let mdast = to_mdast(&markdown, &ParseOptions::gfm()).unwrap();
        let group = Group::from_mdast(mdast);

        println!("{:#?}", group);
    }
}
