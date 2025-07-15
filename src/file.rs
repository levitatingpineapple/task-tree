use std::fmt::{self, Display, Formatter};

use crate::{
    group::Group,
    task::{Task, TaskErr},
    tree::{Child, Parent},
};
use markdown::mdast::Node;

#[derive(Debug, Default)]
pub struct File {
    pub sub_groups: Vec<Group>,
}

impl File {
    pub fn new(node: Node) -> Result<File, FileErr> {
        let mut file = File::default();
        let mut heading_depth = 0;

        if let Some(children) = node.children() {
            for child in children {
                match child {
                    Node::Heading(heading) => {
                        heading_depth = heading.depth;
                        let sub_groups: &mut Vec<Group> = file
                            .last_children(heading_depth - 1)
                            .ok_or(FileErr::HeadingOrder)?;
                        sub_groups.push(Group::new(child.to_string()));
                    }
                    Node::List(list) => {
                        let tasks = Task::new_tasks(list)?;
                        if heading_depth == 0 {
                            return Err(FileErr::LooseTasks);
                        } else {
                            let last_group: &mut Group = file
                                .last_children(heading_depth - 1)
                                .ok_or(FileErr::HeadingOrder)?
                                .last_mut() // Last child
                                .expect(
                                    "Heading depth only incremented - when something is pushed",
                                );
                            last_group.tasks = tasks;
                        }
                    }
                    _ => { /* Ignore other node types */ }
                }
            }
        }
        Ok(file)
    }

    #[allow(unused_variables)]
    pub fn extract_completed_tasks<F>(&mut self, callback: &mut F)
    where
        F: FnMut(Task),
    {
        let mut tasks = Vec::<Task>::new();

        self.for_each_mut(&mut |group| {
            // group.extract_if::<>(&mut { |task: &mut Task| true }, todo!());
            <Group as Parent<Task>>::extract_if(
                group,
                &mut |task| task.done == Some(true), // Predicate
                &mut |task| tasks.push(task),        // Action
            );
        });

        println!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++---");
        println!("{}", self);
        println!("-----------------------------------------------------------------");
        for task in tasks {
            println!("{}", task.text);
        }
    }
}

impl Display for File {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for group in &self.sub_groups {
            write!(f, "{}", group)?
        }
        Ok(())
    }
}

impl Parent<Group> for File {
    fn children(&self) -> &Vec<Group> {
        &self.sub_groups
    }

    fn children_mut(&mut self) -> &mut Vec<Group> {
        &mut self.sub_groups
    }
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum FileErr {
    #[error("Heading parent missing")]
    HeadingOrder,
    #[error("Tasks without group")]
    LooseTasks,
    #[error("Task error: {0}")]
    Ts(#[from] TaskErr),
}

#[cfg(test)]
mod tests {
    use super::*;
    use markdown::{ParseOptions, to_mdast};
    use std::{fs::read_to_string, path::Path};

    #[test]
    fn extract() {
        let path = Path::new("/Users/user/notes/plan/todo.md");
        let markdown = read_to_string(&path).unwrap();
        let node = to_mdast(&markdown, &ParseOptions::gfm()).unwrap();
        let mut file = File::new(node).unwrap();

        file.extract_completed_tasks(&mut |t, c| println!("hey"));
    }
}
