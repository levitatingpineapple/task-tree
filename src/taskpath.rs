use core::fmt;
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use crate::tree;

#[derive(Debug, Clone)]
pub struct TaskPath {
    pub group: tree::Path<String>,
    pub task: tree::Path<String>,
}

impl Display for TaskPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", &self.group, &self.task)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TaskPathErr {
    #[error("Missing separator ':' in task path")]
    MissingSeparator,
    #[error("Invalid group path: {0}")]
    InvalidGroup(String),
    #[error("Invalid task path: {0}")]
    InvalidTask(String),
}

impl FromStr for TaskPath {
    type Err = TaskPathErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (group_str, task_str) = s.split_once(':').ok_or(TaskPathErr::MissingSeparator)?;
        Ok(TaskPath {
            group: group_str
                .parse()
                .map_err(|e: tree::PathErr<_>| TaskPathErr::InvalidGroup(format!("{:?}", e)))?,
            task: task_str
                .parse()
                .map_err(|e: tree::PathErr<_>| TaskPathErr::InvalidTask(format!("{:?}", e)))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_task_path() {
        let path: TaskPath = "group1/subgroup:task1".parse().unwrap();
        assert_eq!(path.group.parent_ids, vec!["group1".to_string()]);
        assert_eq!(path.group.child_id, "subgroup".to_string());
        assert_eq!(path.task.parent_ids, Vec::<String>::new());
        assert_eq!(path.task.child_id, "task1".to_string());

        let err = "group_without_task".parse::<TaskPath>().unwrap_err();
        assert!(matches!(err, TaskPathErr::MissingSeparator));
    }
}
