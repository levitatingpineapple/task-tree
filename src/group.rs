use std::fmt::{self, Display, Formatter};

use chrono::{DateTime, TimeDelta};
use chrono_tz::Tz;

use crate::{
    session::range::Span,
    task::Task,
    tasktree::TotalTime,
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
        if !&self.tasks.is_empty() {
            write!(f, "\n")?;
        }
        for child in &self.sub_groups {
            child.fmt_recursive(f, level + 1)?;
        }
        Ok(())
    }

    pub fn remove_empty(&mut self) {
        for sub_group in &mut self.sub_groups {
            sub_group.remove_empty();
        }
        self.sub_groups.retain(|g| g.is_empty());
    }

    pub fn is_empty(&self) -> bool {
        !self.sub_groups.is_empty() || !self.tasks.is_empty()
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

    fn into_children(self) -> Vec<Group> {
        self.sub_groups
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

    fn into_children(self) -> Vec<Task> {
        self.tasks
    }
}

impl TotalTime for Group {
    fn time_delta(&self, span: Span<DateTime<Tz>>) -> TimeDelta {
        let sub_groups = self
            .sub_groups
            .iter()
            .fold(TimeDelta::zero(), |time, sub_group| {
                time + sub_group.time_delta(span)
            });
        let tasks = self
            .tasks
            .iter()
            .fold(TimeDelta::zero(), |time, task| time + task.time_delta(span));
        tasks + sub_groups
    }
}
