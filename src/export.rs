use crate::task::Task;
use ics::ICalendar;
use markdown::{ParseOptions, mdast::Node, to_mdast};
use std::{
    fs,
    io::{self, Read},
    path::Path,
    process,
};

pub fn export_from(path: &Path) -> io::Result<()> {
    let mut file = fs::File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let ast = to_mdast(&content, &ParseOptions::gfm()).expect("Failed to parse markdown");
    let mut tasks = Vec::new();
    collect_tasks(&ast, &mut tasks, None);
    let mut calendar = ICalendar::new("2.0", "-//Lepi//Task Tree 0.0.1//EN");
    for task in &tasks {
        for event in task.events(&tasks) {
            calendar.add_event(event);
        }
    }
    calendar.save_file(path)?;
    open::that(path)?;
    Ok(())
}

/// Recursively traverses markdown AST,
/// extracts `ListItem` nodes and converts them to tasks
fn collect_tasks(node: &Node, tasks: &mut Vec<Task>, parent: Option<usize>) {
    fn recurse(node: &Node, tasks: &mut Vec<Task>, parent: Option<usize>) {
        if let Some(children) = node.children() {
            for child in children {
                collect_tasks(child, tasks, parent);
            }
        }
    }
    if let Node::ListItem(list_item) = node {
        let task = Task::new(list_item, parent).unwrap_or_else(|err| {
            eprintln!("Error creating task: {:?}", err);
            process::exit(1);
        });
        tasks.push(task);
        // Recursive call with last appended task as parent
        recurse(node, tasks, Some(tasks.len() - 1));
    } else {
        // Recursive call keeping the existing parent
        recurse(node, tasks, parent);
    }
}
