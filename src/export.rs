use crate::{
    group::Group,
    nested::{self, NestedIter},
    session,
    task::Task,
};
use chrono::Local;
use dirs::home_dir;
use ics::{
    Event, ICalendar,
    properties::{RRule, Sequence, Summary},
};
use markdown::{ParseOptions, to_mdast};
use std::{
    fs::{create_dir_all, read_to_string},
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
};

pub fn export_from(md_path: &Path) -> Result<(), ExportErr> {
    let markdown = read_to_string(md_path)?;
    let mdast = to_mdast(&markdown, &ParseOptions::gfm()).map_err(ExportErr::Markdown)?;
    let root_group = Group::from_mdast(mdast)?;

    let mut calendar = ICalendar::new("2.0", "-//Lepi//Task Tree 0.0.1//EN");

    let now = Local::now();
    let dtstamp = session::ics_format(now);

    for group_path in root_group.nested_iter() {
        let iter = group_path.leaf.tasks.iter();
        for task_path in iter.flat_map(|task| task.nested_iter()) {
            for (index, session) in task_path.leaf.sessions.iter().enumerate() {
                let id = event_id(&group_path, &task_path, index);
                let mut event = Event::new(format!("{:x}", id), dtstamp.clone());
                event.push(Summary::new(task_path.leaf.text.clone()));
                event.push(session.dt_start());
                event.push(session.dt_end());
                // Apple calendar will only update event _once_
                // even when `DTSTAMP` and/or `LAST-MODIFIED` are incremented
                // setting sequence number to current unix timestamp
                // allows updating events wihout retaining any state
                event.push(Sequence::new(now.timestamp().to_string()));
                if let Some(rrule) = &session.rrule {
                    event.push(RRule::new(rrule.to_string()));
                }
                calendar.add_event(event);
            }
        }
    }

    let ics_path = home_dir()
        .ok_or(ExportErr::MissingHome)?
        .join(".cache/task-tree/todo.ics");
    let _ = create_dir_all(ics_path.parent().expect("parent"));
    calendar.save_file(&ics_path)?;
    open::that(&ics_path).unwrap();
    Ok(())
}

fn event_id(group_path: &nested::Path<Group>, task_path: &nested::Path<Task>, index: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    for parent in &group_path.parents {
        parent.text.hash(&mut hasher);
    }
    group_path.leaf.text.hash(&mut hasher);
    for parent in &task_path.parents {
        parent.text.hash(&mut hasher);
    }
    task_path.leaf.text.hash(&mut hasher);
    index.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, thiserror::Error)]
pub enum ExportErr {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid Markdown: {0}")]
    Markdown(markdown::message::Message),

    #[error("Missing home directory")]
    MissingHome,

    #[error("Task error: {0}")]
    Task(#[from] crate::task::TaskErr),

    #[error("Group error: {0}")]
    Group(#[from] crate::group::GroupErr),
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn export() {
        export_from(&Path::new("/Users/user/notes/plan/todo.md")).unwrap();
    }
}
