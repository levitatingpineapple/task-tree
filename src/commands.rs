use crate::{
    file::File,
    group::Group,
    session,
    task::Task,
    tree::{Child, Parent},
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

pub fn export_ics_from(md_path: &Path) -> Result<(), ExportErr> {
    let markdown = read_to_string(md_path)?;
    let node = to_mdast(&markdown, &ParseOptions::gfm()).map_err(ExportErr::Markdown)?;
    let file = File::new(node)?;
    let mut calendar = ICalendar::new("2.0", "-//Lepi//Task Tree 0.0.1//EN");
    let now = Local::now();
    // Sequence hack - Apple calendar will only update event _once_(!)
    // even when `DTSTAMP` and/or `LAST-MODIFIED` are incremented
    // setting sequence number to current unix timestamp
    // seems to be the only way to update events wihout retaining any state
    let sequence = Sequence::new(now.timestamp().to_string());
    let datestamp = session::ics_format(&now);
    for group_item in <File as Parent<Group>>::iter(&file) {
        for task_item in <Group as Parent<Task>>::iter(&group_item.child) {
            task_item
                .child
                .sessions
                .iter()
                .enumerate()
                .for_each(|(index, session)| {
                    // Construct unique event id using static hasher
                    let mut hasher = DefaultHasher::new();
                    for parent in &group_item.parent_path {
                        parent.hash(&mut hasher);
                    }
                    group_item.child.id().hash(&mut hasher);
                    for parent in &task_item.parent_path {
                        parent.hash(&mut hasher);
                    }
                    task_item.child.id().hash(&mut hasher);
                    index.hash(&mut hasher);
                    let id = hasher.finish();
                    // Populate and return the event
                    let mut event = Event::new(format!("{:x}", id), datestamp.clone());
                    event.push(Summary::new(task_item.child.text.clone()));
                    event.push(session.dt_start());
                    event.push(session.dt_end());
                    event.push(sequence.clone());
                    if let Some(rrule) = &session.repeat {
                        event.push(RRule::new(rrule.to_string()));
                    }
                    calendar.add_event(event);
                });
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

/// Moves all completed tasks to `todo.md`
pub fn extract_completed(
    todo_path: &Path,
    done_path: &Path,
    dry_run: bool,
) -> Result<(), ExportErr> {
    let mut todo = File::read_from(todo_path)?;
    let mut done = File::read_from(done_path)?;
    todo.extract_completed_tasks(&mut |context| done.insert_task(context));
    todo.remove_empty_groups();
    done.remove_empty_groups();
    done.for_each_mut(&mut |mut group, _| {
        <Group as Parent<Task>>::for_each_mut(&mut group, &mut |task, _| task.done = None);
    });

    if dry_run {
        println!("TODO--------------------------------------------------------TODO");
        println!("{}", todo);
        println!("DONE--------------------------------------------------------DONE");
        println!("{}", done);
        println!("END----------------------------------------------------------END");
    } else {
        std::fs::write(todo_path, todo.to_string()).unwrap();
        std::fs::write(done_path, done.to_string()).unwrap();
    }
    Ok(())
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
    File(#[from] crate::file::FileErr),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // #[ignore]
    fn extract_done_dry_run() {
        extract_completed(
            &Path::new("/Users/user/notes/plan/todo.md"),
            &Path::new("/Users/user/notes/plan/done.md"),
            true,
        )
        .unwrap();
    }

    #[test]
    #[ignore]
    fn extract_done() {
        extract_completed(
            &Path::new("/Users/user/notes/plan/todo.md"),
            &Path::new("/Users/user/notes/plan/done.md"),
            false,
        )
        .unwrap();
    }

    // TODO: Test that sessions are merged
    // TODO: Test that empty groups are removed
    // TODO: Test that merging an existing child is working (perhaps in tree file)
}
