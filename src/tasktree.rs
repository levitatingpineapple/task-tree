use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use crate::{
    group::Group,
    ranged_err::{Ranged, ranged},
    session::{Session, range::Span},
    task::{Task, TaskErr},
    tree::{Child, IteratorItem, Parent},
};
use chrono::{DateTime, TimeDelta};
use chrono_tz::Tz;
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

    pub fn insert_task(&mut self, t: MoveContext) {
        self.insert(t.group_path, Group::new(t.group_id))
            .insert(t.task_path, t.task);
    }

    /// Plucks filtered tasks from the tree
    /// and returns them in the `action` callback
    pub fn pluck_tasks<F, A>(&mut self, filter: &F, action: &mut A)
    where
        A: FnMut(MoveContext),
        F: Fn(&mut Task) -> bool,
    {
        self.for_each_mut(&mut |group, group_path| {
            let group_id = group.id();
            Parent::<Task>::extract_if(
                group,
                &mut vec![],
                &mut |task, task_path| {
                    action(MoveContext {
                        group_path,
                        group_id: group_id.clone(),
                        task_path,
                        task,
                    });
                },
                filter,
            );
        });
    }

    /// Drains the other task-tree from all tasks and inserts then into self
    pub fn union(mut self, mut other: TaskTree) -> TaskTree {
        other.pluck_tasks(&|_| true, &mut |ctx| self.insert_task(ctx));
        self
    }

    /// Removes groups without any tasks or subgroups
    pub fn remove_empty_groups(&mut self) {
        for sub_group in &mut self.groups {
            sub_group.remove_empty();
        }
        self.groups.retain(|g| g.is_empty());
    }

    pub fn _with_sessions<F>(&self, action: &mut F)
    where
        F: FnMut(&IteratorItem<Group>, &IteratorItem<Task>, &Session, usize),
    {
        for group_item in Parent::<Group>::iter(self) {
            for task_item in Parent::<Task>::iter(group_item.child) {
                for (index, session) in task_item.child.sessions.iter().enumerate() {
                    action(&group_item, &task_item, &session, index);
                }
            }
        }
    }
}

/// A type, which occupies an amount of time in a given time range
pub trait TotalTime {
    fn time_delta(&self, span: Span<DateTime<Tz>>) -> TimeDelta;
}

impl TotalTime for TaskTree {
    fn time_delta(&self, span: Span<DateTime<Tz>>) -> TimeDelta {
        self.groups.iter().fold(TimeDelta::zero(), |time, group| {
            time + group.time_delta(span)
        })
    }
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

/// Task with all of the relavant context
/// for moving it to another tree
pub struct MoveContext<'a> {
    pub group_path: &'a Vec<String>,
    pub group_id: String,
    pub task_path: &'a Vec<String>,
    pub task: Task,
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
