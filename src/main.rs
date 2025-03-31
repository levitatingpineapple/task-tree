mod session;
mod task;

use ics::ICalendar;
use markdown::{ParseOptions, mdast::Node, to_mdast};
use std::{
    env, fs,
    io::{self, Read},
    path::Path,
    process,
};
use task::Task;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        process::exit(1);
    }

    let file_path = &args[1];
    let path = Path::new(file_path);

    // Read the file content
    let mut file = fs::File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    // Parse the markdown content
    let ast = to_mdast(&content, &ParseOptions::gfm()).expect("Failed to parse markdown");

    // Find ListItem nodes and create Task structs
    let mut tasks = Vec::new();
    collect_tasks(&ast, &mut tasks);

    // Create calendar
    let mut calendar = ICalendar::new("2.0", "-//Lepi//Task Tree 0.0.1//EN");
    for task in &tasks {
        for event in task.events() {
            calendar.add_event(event);
        }
    }
    calendar.save_file("todo.ics")?;
    Ok(())
}

/// Recursively traverses markdown AST,
/// extracts `ListItem` nodes and converts them to tasks
fn collect_tasks(node: &Node, tasks: &mut Vec<Task>) {
    if let Node::ListItem(list_item) = node {
        match Task::new(list_item) {
            Ok(task) => tasks.push(task),
            Err(err) => {
                eprintln!("Error creating task: {:?}", err);
                process::exit(1);
            }
        }
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_tasks(child, tasks);
        }
    }
}
