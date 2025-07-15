use std::fmt::{self, Display, Formatter};

use crate::{
    task::Task,
    tree::{Child, Parent},
};

#[derive(Debug, Default, PartialEq)]
pub struct Group {
    pub text: String,
    pub sub_groups: Vec<Group>,
    pub tasks: Vec<Task>,
}

impl Group {
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
        // Headings start at level 1
        self.fmt_recursive(f, 1)
    }
}

impl Parent<Group> for Group {
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

impl Parent<Task> for Group {
    fn children(&self) -> &Vec<Task> {
        &self.tasks
    }

    fn children_mut(&mut self) -> &mut Vec<Task> {
        &mut self.tasks
    }
}
