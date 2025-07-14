use std::fmt::{self, Display, Formatter};

use crate::{
    nested::NestedIter,
    task::{self, Task, TaskErr},
};
use markdown::mdast::Node;

#[derive(Debug, Default, PartialEq)]
pub struct Group {
    pub text: String,
    pub sub_groups: Vec<Group>,
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
                        foo.sub_groups.push(Group::new(child.to_string()));
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
    fn new<S: Into<String>>(text: S) -> Self {
        Group {
            text: text.into(),
            ..Default::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.sub_groups.is_empty() && self.tasks.is_empty()
    }

    /// Returns last subcategory at some level
    fn last_sub(&mut self, depth: u8) -> Option<&mut Group> {
        if depth == 0 {
            Some(self)
        } else {
            self.sub_groups.last_mut().map(|s| s.last_sub(depth - 1))?
        }
    }

    /// Assuming tree is built depth-first returns last added element
    fn last_added(&mut self) -> &mut Group {
        if self.sub_groups.is_empty() {
            self
        } else {
            // Unwrap required due to borrow checker..
            self.sub_groups.last_mut().unwrap().last_added()
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
        for child in &self.sub_groups {
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
        &self.sub_groups
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

impl Group {
    pub fn extract_completed_tasks<F: FnMut(Task, &task::Context)>(&mut self, callback: &mut F) {
        self.ect(&mut task::Context::default(), callback, true);
    }

    fn ect<F: FnMut(Task, &task::Context)>(
        &mut self,
        context: &mut task::Context,
        callback: &mut F,
        root: bool,
    ) {
        // Exclude root node from he context.
        if !root {
            context.groups.push(self.text.clone());
        }
        // Extract all root tasks
        for task in self.tasks.extract_if(.., |t| t.done == Some(true)) {
            callback(task, context);
        }
        // Recurse into child tasks
        for task in self.tasks.iter_mut() {
            task.extract_completed(context, callback);
        }
        // Repeat for all children - discarding the empty ones
        self.sub_groups.retain_mut(|child| {
            child.ect(context, callback, false);
            !child.is_empty()
        });
        context.groups.pop();
    }

    // pub fn insert_task(&mut self, task: Task, mut context: task::Context) {
    // if let Some(group_id) = context.groups.drain(..1).next() {
    //     let group =
    // } else {

    //     if let Some(task_id) = context.tasks.drain(..1).next() {
    //         // Recursive task insert
    //     } else {
    //         // This is a root task - add to group
    //         self.tasks.push(task);
    //     }
    // }
    // }
}
