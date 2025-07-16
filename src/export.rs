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

pub fn export_from(md_path: &Path) -> Result<(), ExportErr> {
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
pub fn extract_completed(path: &Path) -> Result<(), ExportErr> {
    let markdown = read_to_string(&path).unwrap();
    let node = to_mdast(&markdown, &ParseOptions::gfm())
        .map_err(ExportErr::Markdown)
        .expect("TEST - Remove this");
    let mut file = File::new(node).unwrap();
    let mut extracted = Vec::<Task>::new();

    // file.extract_completed_tasks(&mut |t| extracted.push(t));
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
    fn display() {
        let path = Path::new("/Users/user/notes/plan/todo.md");
        let markdown = read_to_string(&path).unwrap();
        let node = to_mdast(&markdown, &ParseOptions::gfm())
            .map_err(ExportErr::Markdown)
            .unwrap();
        let mut file = File::new(node);

        dbg!(file);
        // let mut extracted = Vec::<(Task, task::Context)>::new();
        // root.extract_completed_tasks(&mut |t, c| extracted.push((t, c.clone())));

        // let string = root.to_string();
        // println!("-----------------------------------------------------------------");
        // println!("{}", string);

        // println!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
        // for (task, context) in extracted {
        //     println!("CONTEXT::{:?}", context);
        //     println!("TASK::{}", task);
        //     println!("~~~~~~~~~")
        // }
    }
}
