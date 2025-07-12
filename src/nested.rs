use std::slice::{Iter, from_ref};

pub trait NestedIter: Sized {
    fn children(&self) -> &Vec<Self>;

    /// Nested iterator is a depth-first iterator that
    /// visits all nodes while prividing full path to parent nodes
    /// Parents are ordered starting from root
    fn nested_iter(&self) -> DFI<'_, Self> {
        DFI {
            nodes: vec![],
            iterators: vec![from_ref(self).iter()],
        }
    }
}

#[derive(Debug)]
pub struct Path<'a, T> {
    pub leaf: &'a T,
    pub parents: Vec<&'a T>,
}

pub struct DFI<'a, T: NestedIter> {
    nodes: Vec<&'a T>,
    iterators: Vec<Iter<'a, T>>,
}

impl<'a, T: NestedIter> Iterator for DFI<'a, T> {
    type Item = Path<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(children_iter) = self.iterators.last_mut() {
            if let Some(child_ref) = children_iter.next() {
                let item = Path {
                    leaf: child_ref,
                    parents: self.nodes.clone(),
                };
                self.nodes.push(child_ref);
                self.iterators.push(child_ref.children().iter());
                Some(item)
            } else {
                self.nodes.pop();
                self.iterators.pop();
                self.next() // Recursive call while backtracking
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use std::fmt;

    #[test]
    fn iterator() {
        #[rustfmt::skip]
        let tree = node("Root", vec![
            node("Foo", vec![
                node("FooA", vec![
                    node("FooA1", vec![]),
                    node("FooA2", vec![]),
                ]),
                node("FooB", vec![
                    node("FooB1", vec![]),
                ]),
            ]),
            node("Bar", vec![]),
        ]);
        let display = tree
            .nested_iter()
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join("\n")
            + "\n";
        let expectation = indoc! {"
            [Root]
            Root->[Foo]
            Root->Foo->[FooA]
            Root->Foo->FooA->[FooA1]
            Root->Foo->FooA->[FooA2]
            Root->Foo->[FooB]
            Root->Foo->FooB->[FooB1]
            Root->[Bar]
        "};
        assert_eq!(expectation, display);
    }

    struct Node {
        text: String,
        children: Vec<Node>,
    }

    impl NestedIter for Node {
        fn children(&self) -> &Vec<Self> {
            &self.children
        }
    }

    impl fmt::Display for super::Path<'_, Node> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for parent in self.parents.iter() {
                write!(f, "{}->", parent.text)?;
            }
            write!(f, "[{}]", self.leaf.text)?;
            Ok(())
        }
    }

    fn node(text: &str, children: Vec<Node>) -> Node {
        Node {
            text: text.to_string(),
            children,
        }
    }
}
