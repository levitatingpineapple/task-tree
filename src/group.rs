use std::fmt::{self, Display, Formatter};

use crate::{
    nested::NestedIter,
    task::{self, Task, TaskErr},
    tree::{Child, Root},
};
use markdown::mdast::Node;

#[derive(Debug, Default)]
pub struct File {
    pub sub_groups: Vec<Group>,
    pub tasks: Vec<Task>,
}

impl File {
    pub fn new(node: Node) -> Result<File, GroupErr> {
        let mut file = File::default();
        let mut group_level = 0;

        if let Some(children) = node.children() {
            for child in children {
                match child {
                    Node::Heading(heading) => {
                        group_level = heading.depth - 1;
                        let sub_groups: &mut Vec<Group> = file
                            .last_children(group_level)
                            .ok_or(GroupErr::HeadingOrder)?;
                        sub_groups.push(Group::new(child.to_string()));
                    }
                    Node::List(list) => {
                        let tasks = Task::new_tasks(list)?;
                        if group_level == 0 {
                            // Tasks without group
                            file.tasks = tasks;
                        } else {
                            // For level `1` this will be last sub_group of the `File`
                            let last_group: &mut Group = file
                                .last_children(group_level - 1)
                                .ok_or(GroupErr::HeadingOrder)?
                                .last_mut() // Last child
                                .unwrap(); //
                            last_group.tasks = tasks;
                        }
                    }
                    _ => { /* Ignore other node types */ }
                }
            }
        }
        Ok(file)
    }
}

impl Root<Group> for File {
    fn children(&self) -> &Vec<Group> {
        &self.sub_groups
    }

    fn children_mut(&mut self) -> &mut Vec<Group> {
        &mut self.sub_groups
    }
}

impl Root<Task> for File {
    fn children(&self) -> &Vec<Task> {
        &self.tasks
    }

    fn children_mut(&mut self) -> &mut Vec<Task> {
        &mut self.tasks
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Group {
    pub text: String,
    pub sub_groups: Vec<Group>,
    pub tasks: Vec<Task>,
}

impl Group {
    // TODO: Remove (this should be from trait)
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

impl Root<Group> for Group {
    fn children(&self) -> &Vec<Self> {
        &self.sub_groups
    }

    fn children_mut(&mut self) -> &mut Vec<Self> {
        &mut self.sub_groups
    }
}

impl Child for Group {
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

impl Root<Task> for Group {
    fn children(&self) -> &Vec<Task> {
        &self.tasks
    }

    fn children_mut(&mut self) -> &mut Vec<Task> {
        &mut self.tasks
    }
}

// TODO: Remove
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
}
