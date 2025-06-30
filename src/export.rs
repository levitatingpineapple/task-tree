use crate::task::Task;
use dirs::home_dir;
use ics::ICalendar;
use markdown::{ParseOptions, mdast::Node, to_mdast};
use std::{
    fs::{create_dir_all, read_to_string},
    path::Path,
};

pub fn export_from(md_path: &Path) -> Result<(), ExportErr> {
    let md_syntax_tree =
        to_mdast(&read_to_string(md_path)?, &ParseOptions::gfm()).map_err(ExportErr::Markdown)?;
    let mut tasks = Vec::new();
    collect_tasks_recursively(&md_syntax_tree, &mut tasks, None)?;
    let mut calendar = ICalendar::new("2.0", "-//Lepi//Task Tree 0.0.1//EN");
    for task in &tasks {
        // TODO: Here we should pass in the context of the task (groups, parent tasks)
        for event in task.events() {
            calendar.add_event(event);
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

/// Recursively traverses markdown AST,
/// extracts `ListItem` nodes and converts them to tasks
fn collect_tasks_recursively(
    node: &Node,
    tasks: &mut Vec<Task>,
    parent: Option<usize>,
) -> Result<(), ExportErr> {
    fn traverse_children(
        node: &Node,
        tasks: &mut Vec<Task>,
        parent: Option<usize>,
    ) -> Result<(), ExportErr> {
        if let Some(children) = node.children() {
            for child in children {
                collect_tasks_recursively(child, tasks, parent)?;
            }
        }
        Ok(())
    }
    if let Node::ListItem(list_item) = node {
        let task = Task::new(list_item)?;
        tasks.push(task);
        // Recursive call with last appended task as parent
        traverse_children(node, tasks, Some(tasks.len() - 1))?;
    } else {
        // Recursive call keeping the existing parent
        traverse_children(node, tasks, parent)?;
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
}

#[cfg(test)]
mod tests {
    use crate::tree::Group;

    use super::*;

    #[test]
    fn print_tree() {
        let mdast = to_mdast(
            &read_to_string("/Users/user/notes/plan/todo.md").unwrap(),
            &ParseOptions::gfm(),
        )
        .unwrap();
        let tree = Group::from_mdast(mdast);
        println!("{:#?}", tree);
    }
}
