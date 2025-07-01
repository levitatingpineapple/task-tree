use std::slice::{Iter, from_ref};

pub trait Nested: Sized {
    fn children(&self) -> &Vec<Self>;

    fn nested_iter(&self) -> DFI<Self> {
        DFI {
            nodes: vec![],
            iterators: vec![from_ref(self).iter()],
        }
    }
}

#[derive(Debug)]
pub struct Foo<'a, T> {
    leaf: &'a T,
    parents: Vec<&'a T>,
}

pub struct DFI<'a, T: Nested> {
    nodes: Vec<&'a T>,
    iterators: Vec<Iter<'a, T>>,
}

impl<'a, T: Nested> Iterator for DFI<'a, T> {
    type Item = Foo<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(children_iter) = self.iterators.last_mut() {
            if let Some(child_ref) = children_iter.next() {
                let item = Foo {
                    leaf: child_ref,
                    parents: self.nodes.clone(),
                };
                self.nodes.push(child_ref);
                self.iterators.push(child_ref.children().iter());
                Some(item)
            } else {
                self.nodes.pop();
                self.iterators.pop();
                self.next() // Recursive call, while backtracking
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use indoc::indoc;

    use super::*;

    struct Node {
        text: String,
        children: Vec<Node>,
    }

    impl Nested for Node {
        fn children(&self) -> &Vec<Self> {
            &self.children
        }
    }

    impl fmt::Display for super::Foo<'_, Node> {
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
}
