use std::{
    fmt::{self, Display, Formatter},
    fs::read_to_string,
    path::Path,
};

use crate::{
    group::Group,
    task::{Task, TaskErr},
    tree::{Child, Parent},
};
use markdown::{ParseOptions, mdast::Node, to_mdast};

#[derive(Debug, Default)]
pub struct File {
    pub sub_groups: Vec<Group>,
}

impl File {
    pub fn read_from(path: &Path) -> Result<File, FileErr> {
        let markdown = read_to_string(&path).unwrap();
        File::new(to_mdast(&markdown, &ParseOptions::gfm()).map_err(|m| FileErr::Markdown(m))?)
    }

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

    pub fn insert_task(&mut self, t: Context) {
        self.insert(t.group_path, t.group)
            .insert(t.task_path, t.task);
    }

    pub fn extract_completed_tasks<F>(&mut self, action: &mut F)
    where
        F: FnMut(Context),
    {
        self.for_each_mut(&mut |group, group_path| {
            let group_id = group.id();
            <Group as Parent<Task>>::extract_if(
                group,
                &mut vec![],
                &mut |task, task_path| {
                    action(Context {
                        group_path,
                        group: Group::new(group_id.clone()), // New shallow group
                        task_path,
                        task,
                    });
                },
                &|task| task.done == Some(true),
            );
        });
    }

    pub fn remove_empty_groups(&mut self) {
        for sub_group in &mut self.sub_groups {
            sub_group.remove_empty();
        }
        self.sub_groups.retain(|g| g.is_empty());
    }
}

pub struct Context<'a> {
    pub group_path: &'a Vec<String>,
    pub group: Group,
    pub task_path: &'a Vec<String>,
    pub task: Task,
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

    fn into_children(self) -> Vec<Group> {
        self.sub_groups
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FileErr {
    #[error("Heading parent missing")]
    HeadingOrder,
    #[error("Tasks without group")]
    LooseTasks,
    #[error("Task error: {0}")]
    Task(#[from] TaskErr),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid Markdown: {0}")]
    Markdown(markdown::message::Message),
}
