use crate::{
    group::Group,
    task::{self, Task, TaskErr},
    tree::{Child, Root},
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

    pub fn extract_completed_tasks<F: FnMut(Task, &task::Context)>(&mut self, callback: &mut F) {
        self.ect(&mut task::Context::default(), callback, true);
    }

    fn ect<F: FnMut(Task, &task::Context)>(
        &mut self,
        context: &mut task::Context,
        callback: &mut F,
        root: bool,
    ) {
        todo!()
        //     // Exclude root node from he context.
        //     if !root {
        //         context.groups.push(self.text.clone());
        //     }
        //     // Extract all root tasks
        //     for task in self.tasks.extract_if(.., |t| t.done == Some(true)) {
        //         callback(task, context);
        //     }
        //     // Recurse into child tasks
        //     for task in self.tasks.iter_mut() {
        //         task.extract_completed(context, callback);
        //     }
        //     // Repeat for all children - discarding the empty ones
        //     self.sub_groups.retain_mut(|child| {
        //         child.ect(context, callback, false);
        //         !child.is_empty()
        //     });
        //     context.groups.pop();
    }
}

impl Root<Group> for File {
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
