use crate::{
    context::{CalDAV, Context},
    file::File,
    group::Group,
    session,
    task::Task,
    tree::{Child, Parent},
};
use chrono::Local;
use ics::{
    Event, ICalendar,
    properties::{RRule, Sequence, Summary},
};
use markdown::{ParseOptions, to_mdast};
use reqwest;
use std::{
    fmt::Debug,
    fs::read_to_string,
    hash::{DefaultHasher, Hash, Hasher},
};

async fn upload(
    uid: String,
    client: &reqwest::Client,
    calendar: ICalendar<'_>,
    caldav: &CalDAV,
) -> Result<(), ExportErr> {
    let url = format!("{}/{}.ics", caldav.url, uid);
    let calendar_data = calendar.to_string();
    let status = client
        .put(&url)
        .basic_auth(&caldav.user, Some(&caldav.pass))
        .header("Content-Type", "text/calendar; charset=utf-8")
        .body(calendar_data)
        .send()
        .await?
        .status();
    if status.is_success() {
        Ok(())
    } else {
        Err(ExportErr::CalDAV(format!(
            "Upload failed with status: {}",
            status
        )))
    }
}

pub async fn export_ics(context: &Context) -> Result<(), ExportErr> {
    let markdown = read_to_string(&context.todo())?;
    let node = to_mdast(&markdown, &ParseOptions::gfm()).map_err(ExportErr::Markdown)?;
    let file = File::new(node)?;
    let now = Local::now();
    let http_client = reqwest::Client::new();
    // Sequence hack - Apple calendar will only update event _once_(!)
    // even when `DTSTAMP` and/or `LAST-MODIFIED` are incremented
    // setting sequence number to current unix timestamp
    // seems to be the only way to update events wihout retaining any state
    let sequence = Sequence::new(now.timestamp().to_string());
    let datestamp = session::ics_format(&now);
    for group_item in <File as Parent<Group>>::iter(&file) {
        for task_item in <Group as Parent<Task>>::iter(&group_item.child) {
            for (index, session) in task_item.child.sessions.iter().enumerate() {
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
                let uid = format!("{:x}", hasher.finish());
                // Populate and return the event
                let mut event = Event::new(uid.clone(), datestamp.clone());
                event.push(Summary::new(task_item.child.text.clone()));
                event.push(session.dt_start());
                event.push(session.dt_end());
                event.push(sequence.clone());
                if let Some(repeat) = &session.repeat {
                    event.push(RRule::new(repeat.rule.to_string()));
                }
                let mut calendar = ICalendar::new("2.0", "-//Lepi//Task Tree 0.0.1//EN");
                calendar.add_event(event);
                upload(uid, &http_client, calendar, &context.calendar()).await?;
            }
        }
    }
    Ok(())
}

/// Moves all completed tasks to `todo.md`
pub fn extract_completed(context: &Context) -> Result<(), ExportErr> {
    let todo_path = context.todo();
    let done_path = context.done();
    let mut todo = File::read_from(&todo_path)?;
    let mut done = File::read_from(&done_path)?;
    todo.extract_completed_tasks(&mut |context| done.insert_task(context));
    todo.remove_empty_groups();
    done.remove_empty_groups();
    done.for_each_mut(&mut |mut group, _| {
        <Group as Parent<Task>>::for_each_mut(&mut group, &mut |task, _| task.done = None);
    });
    std::fs::write(&todo_path, todo.to_string()).unwrap();
    std::fs::write(&done_path, done.to_string()).unwrap();
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ExportErr {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid Markdown: {0}")]
    Markdown(markdown::message::Message),
    #[error("Task error: {0}")]
    Task(#[from] crate::task::TaskErr),
    #[error("Group error: {0}")]
    File(#[from] crate::file::FileErr),
    #[error("HTTP request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("CalDAV error: {0}")]
    CalDAV(String),
}
