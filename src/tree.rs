#![allow(dead_code, unused_variables)]
use crate::task::{Task, TaskErr};
use markdown::mdast::Node;

#[derive(Debug, PartialEq, Default)]
pub struct Group {
    text: String,
    sub: Vec<Group>,
    tasks: Vec<Task>,
}

impl Group {
    pub fn from_mdast(mdast: Node) -> Result<Self, TreeErr> {
        let mut root = Group::new("Main");
        build_tree(&mdast, &mut root)?;
        Ok(root)
    }

    pub fn new<S: Into<String>>(text: S) -> Self {
        Group {
            text: text.into(),
            ..Default::default()
        }
    }

    pub fn last_sub(&mut self, depth: u8) -> Option<&mut Group> {
        if depth == 0 {
            Some(self)
        } else {
            self.sub.last_mut().map(|s| s.last_sub(depth - 1))?
        }
    }

    pub fn last_added(&mut self) -> &mut Group {
        if self.sub.is_empty() {
            self
        } else {
            self.sub.last_mut().unwrap().last_added()
        }
    }
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum TreeErr {
    #[error("Heading parent missing")]
    HeadingOrder,
    #[error("Task error: {0}")]
    Ts(#[from] TaskErr),
}

fn build_tree(node: &Node, root: &mut Group) -> Result<(), TreeErr> {
    match node {
        Node::Heading(heading) => {
            let foo = root
                .last_sub(heading.depth - 1)
                .ok_or(TreeErr::HeadingOrder)?;
            foo.sub.push(Group::new(node.to_string()));
        }
        Node::ListItem(list_item) => {
            let task = Task::new(&list_item, None)?;
            root.last_added().tasks.push(task);
            // TODO: Probably no need to recurse here - list item should initialise with all it's children?
        }

        _ => {
            // println!("ignore node")
        }
    }
    if let Some(children) = node.children() {
        for child in children {
            build_tree(child, root)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use markdown::{ParseOptions, to_mdast};

    use super::*;

    #[test]
    fn new_tree() {
        let markdown = indoc! {r#"
            # One
            - [ ] Task1
            - [x] Task2
            ## SubA
            ### Nested
            ## SubB
            - [ ] WORK
            # Two
        "#};

        let mdast = to_mdast(&markdown, &ParseOptions::gfm()).unwrap();
        let group = Group::from_mdast(mdast);

        println!("{:#?}", group);
    }
}
