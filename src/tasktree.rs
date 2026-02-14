use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use crate::{
    group::Group,
    ranged::{Ranged, ranged},
    task::{Task, TaskErr},
    tree::{Child, Parent},
};
use markdown::{
    ParseOptions,
    mdast::Node,
    message::{Message, Place},
    to_mdast,
    unist::Position,
};

#[derive(Debug, Default)]
pub struct TaskTree {
    pub groups: Vec<Group>,
}

impl TaskTree {
    pub fn new(node: Node) -> Result<TaskTree, Ranged<TaskTreeErr>> {
        let mut file = TaskTree::default();
        let mut heading_depth = 0;

        if let Some(children) = node.children() {
            for child in children {
                match child {
                    Node::Heading(heading) => {
                        heading_depth = heading.depth;
                        let sub_groups: &mut Vec<Group> = file
                            .last_children(heading_depth - 1)
                            .ok_or(ranged(TaskTreeErr::HeadingOrder, heading.position.as_ref()))?;
                        sub_groups.push(Group::new(child.to_string()));
                    }
                    Node::List(list) => {
                        let tasks = Task::new_tasks(list, None)?;
                        if heading_depth == 0 {
                            return Err(ranged(TaskTreeErr::LooseTasks, list.position.as_ref()));
                        } else {
                            let last_group: &mut Group = file
                                .last_children(heading_depth - 1)
                                .ok_or(ranged(TaskTreeErr::HeadingOrder, list.position.as_ref()))?
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
        for sub_group in &mut self.groups {
            sub_group.remove_empty();
        }
        self.groups.retain(|g| g.is_empty());
    }
}

pub struct Context<'a> {
    pub group_path: &'a Vec<String>,
    pub group: Group,
    pub task_path: &'a Vec<String>,
    pub task: Task,
}

impl Display for TaskTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for group in &self.groups {
            write!(f, "{}", group)?
        }
        Ok(())
    }
}

impl FromStr for TaskTree {
    type Err = Ranged<TaskTreeErr>;

    fn from_str(str: &str) -> Result<TaskTree, Ranged<TaskTreeErr>> {
        TaskTree::new(to_mdast(&str, &ParseOptions::gfm())?)
    }
}

impl Parent<Group> for TaskTree {
    fn children(&self) -> &Vec<Group> {
        &self.groups
    }

    fn children_mut(&mut self) -> &mut Vec<Group> {
        &mut self.groups
    }

    fn into_children(self) -> Vec<Group> {
        self.groups
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TaskTreeErr {
    #[error("Heading parent missing")]
    HeadingOrder,
    #[error("Tasks without group")]
    LooseTasks,
    #[error("Task error: {0}")]
    Task(TaskErr),
    #[error("Invalid Markdown: {0}")]
    Markdown(String),
}

impl From<Ranged<TaskErr>> for Ranged<TaskTreeErr> {
    fn from(task_err: Ranged<TaskErr>) -> Ranged<TaskTreeErr> {
        Ranged {
            error: TaskTreeErr::Task(task_err.error),
            range: task_err.range,
        }
    }
}

impl From<Message> for Ranged<TaskTreeErr> {
    fn from(message: Message) -> Ranged<TaskTreeErr> {
        ranged(
            TaskTreeErr::Markdown(message.reason),
            message
                .place
                .map(|place| match *place {
                    Place::Position(position) => position,
                    Place::Point(point) => Position {
                        start: point.clone(),
                        end: point,
                    },
                })
                .as_ref(),
        )
    }
}
