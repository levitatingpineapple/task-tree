use core::fmt;
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use crate::tree;

#[derive(Debug, Clone, PartialEq)]
pub struct TaskPath {
    pub group: tree::Path<String>,
    pub task: tree::Path<String>,
}

fn write_segment(f: &mut Formatter<'_>, s: &str) -> fmt::Result {
    if s.contains('/') || s.contains(':') {
        write!(f, "\"{}\"", s)
    } else {
        write!(f, "{s}")
    }
}

fn write_path(f: &mut Formatter<'_>, path: &tree::Path<String>) -> fmt::Result {
    for parent in &path.parent_ids {
        write_segment(f, parent)?;
        write!(f, "/")?;
    }
    write_segment(f, &path.child_id)
}

impl Display for TaskPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write_path(f, &self.group)?;
        write!(f, ":")?;
        write_path(f, &self.task)
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

fn split_unquoted(s: &str, sep: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' => in_quotes = !in_quotes,
            c if c == sep && !in_quotes => {
                parts.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

fn unquote(s: &str) -> String {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn parse_path(s: &str) -> Result<tree::Path<String>, String> {
    if s.is_empty() {
        return Err("empty path".to_string());
    }
    let segments = split_unquoted(s, '/');
    let (last, parents) = segments.split_last().unwrap();
    Ok(tree::Path {
        parent_ids: parents.iter().map(|p| unquote(p)).collect(),
        child_id: unquote(last),
    })
}

impl FromStr for TaskPath {
    type Err = TaskPathErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = split_unquoted(s, ':');
        if parts.len() != 2 {
            return Err(TaskPathErr::MissingSeparator);
        }
        Ok(TaskPath {
            group: parse_path(parts[0]).map_err(TaskPathErr::InvalidGroup)?,
            task: parse_path(parts[1]).map_err(TaskPathErr::InvalidTask)?,
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

    #[test]
    fn parse_escaped_path() {
        let tp = TaskPath {
            group: tree::Path {
                parent_ids: vec![],
                child_id: "group".to_string(),
            },
            task: tree::Path {
                parent_ids: vec![],
                child_id: "[task](https://task.com)".to_string(),
            },
        };
        let string = tp.to_string();

        assert_eq!(string, "group:\"[task](https://task.com)\"");

        let decoded = TaskPath::from_str(&string).unwrap();
        assert_eq!(tp, decoded);
    }
}
