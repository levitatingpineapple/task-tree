use crate::{group::Group, nested::Nested, task::Task};
use dirs::home_dir;
use ics::ICalendar;
use markdown::{ParseOptions, to_mdast};
use std::{
    fs::{create_dir_all, read_to_string},
    path::Path,
};

pub fn export_from(md_path: &Path) -> Result<(), ExportErr> {
    let markdown = read_to_string(md_path)?;
    let mdast = to_mdast(&markdown, &ParseOptions::gfm()).map_err(ExportErr::Markdown)?;
    let root_group = Group::from_mdast(mdast)?;
    let mut calendar = ICalendar::new("2.0", "-//Lepi//Task Tree 0.0.1//EN");
    for group in root_group.nested_iter() {
        for root_task in &group.leaf.tasks {
            for task in root_task.nested_iter() {
                println!("🔴{}", task.leaf.text)
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
    use crate::group::Group;

    use super::*;

    #[test]
    fn test_export() {
        // let mdast = to_mdast(
        //     &read_to_string("/Users/user/notes/plan/todo.md").unwrap(),
        //     &ParseOptions::gfm(),
        // )
        // .unwrap();
        // let group = Group::from_mdast(mdast);

        // for a in group.iter() {
        //     println!("{}", a.text)
        // }

        // println!("{:#?}", group);
        export_from(&Path::new("/Users/user/notes/plan/todo.md")).unwrap();
    }
}
