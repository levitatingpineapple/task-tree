use crate::{
    context::{CalDAV, Context},
    group::Group,
    ranged_err::Ranged,
    session,
    task::Task,
    tasktree::{TaskTree, TaskTreeErr},
    tree::Parent,
};
use chrono::Utc;
use ics::{
    Event, ICalendar,
    properties::{LastModified, RRule, Summary},
};
use reqwest;
use std::{
    fmt::Debug,
    fs::read_to_string,
    hash::{DefaultHasher, Hash, Hasher},
    str::FromStr,
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
    let tasktree = TaskTree::from_str(&markdown)?;
    let now = Utc::now();
    let http_client = reqwest::Client::new();
    let datestamp = session::ics_format(&now);
    for group_item in <TaskTree as Parent<Group>>::iter(&tasktree) {
        for task_item in <Group as Parent<Task>>::iter(&group_item.child) {
            for (index, session) in task_item.child.sessions.iter().enumerate() {
                // Construct unique event id using static hasher
                let mut hasher = DefaultHasher::new();
                group_item.id_hash(&mut hasher);
                task_item.id_hash(&mut hasher);
                index.hash(&mut hasher);
                let uid = format!("{:x}", hasher.finish());
                // Populate and return the event
                let mut event = Event::new(uid.clone(), datestamp.clone());
                event.push(Summary::new(task_item.child.text.clone()));
                event.push(session.dt_start());
                event.push(session.dt_end());
                event.push(LastModified::new(datestamp.clone()));
                if let Some(repeat) = &session.repeat {
                    event.push(RRule::new(repeat.rule.to_string()));
                }
                let mut calendar = ICalendar::new("2.0", "-//Lepi//Task Tree 0.0.1//EN");
                calendar.add_event(event);
                upload(uid, &http_client, calendar, &context.config().caldav).await?;
            }
        }
    }
    Ok(())
}

/// Moves all completed tasks from `todo.md` to `done.md`
pub fn extract_completed(context: &Context) -> Result<(), ExportErr> {
    let todo_path = context.todo();
    let done_path = context.done();
    let mut todo = TaskTree::from_str(&read_to_string(&todo_path)?)?;
    let mut done = TaskTree::from_str(&read_to_string(&done_path)?)?;
    _extract_completed(&mut todo, &mut done);
    std::fs::write(&todo_path, todo.to_string()).unwrap();
    std::fs::write(&done_path, done.to_string()).unwrap();
    Ok(())
}

fn _extract_completed(todo: &mut TaskTree, done: &mut TaskTree) {
    todo.pluck_tasks(&|task| task.done == Some(true), &mut |context| {
        dbg!(&context.task_path, &context.task.text);
        done.insert_task(context)
    });
    todo.remove_empty_groups();
    done.remove_empty_groups();
    done.for_each_mut(&mut |mut group, _| {
        <Group as Parent<Task>>::for_each_mut(&mut group, &mut |task, _| task.done = None);
    });
}

#[derive(Debug, thiserror::Error)]
pub enum ExportErr {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File error: {0}")]
    TaskTree(#[from] Ranged<TaskTreeErr>),
    #[error("HTTP request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("CalDAV error: {0}")]
    CalDAV(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasktree::TaskTree;
    use indoc::indoc;
    use std::str::FromStr;

    #[test]
    fn extract_completed() {
        let todo_str = indoc! {"
            # Task Tree
            
            - [ ] Workspace Functions
              - [x] iCal export - run on save
              - [x] Todo to Done archiving function
            - [ ] Diagnostics `26/02/13_20-21` `26/02/14_12-13`
              - [x] Children of done must be done `26/02/14_18-19`
        "};
        let mut todo = TaskTree::from_str(todo_str).unwrap();
        let mut done = TaskTree::from_str("").unwrap();
        _extract_completed(&mut todo, &mut done);
        let done_str = indoc! {"
            # Task Tree
            
            - Workspace Functions
              - iCal export - run on save
              - Todo to Done archiving function
            - Diagnostics
              - Children of done must be done `26/02/14_18-19`
        "};

        assert_eq!(&done.to_string(), done_str);
    }

    // #[test]
    // fn foo() {
    //     let string = read_to_string("/home/user/sync/notes/plan/todo.md").unwrap();
    //     let todo = TaskTree::from_str(&string).unwrap();

    //     print!("{}", todo.to_string());
    // }
}
