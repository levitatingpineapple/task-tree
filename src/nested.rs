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

pub struct DFI<'a, T: Nested> {
    nodes: Vec<&'a T>,
    iterators: Vec<Iter<'a, T>>,
}

pub struct Element<'a, T> {
    leaf: &'a T,
    parents: Vec<&'a T>,
}

impl<'a, T: Nested> Iterator for DFI<'a, T> {
    type Item = Vec<&'a T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(children_iter) = self.iterators.last_mut() {
            if let Some(child_ref) = children_iter.next() {
                self.nodes.push(child_ref);
                self.iterators.push(child_ref.children().iter());
                Some(self.nodes.clone())
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

    fn node(text: &str, children: Vec<Node>) -> Node {
        Node {
            text: text.to_string(),
            children,
        }
    }

    fn iter_result(node: Node) -> Vec<Vec<String>> {
        node.nested_iter()
            .map(|level| level.iter().map(|node| node.text.clone()).collect())
            .collect()
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
        let expect = vec![
            vec!["Root"],
            vec!["Root", "Foo"],
            vec!["Root", "Foo", "FooA"],
            vec!["Root", "Foo", "FooA", "FooA1"],
            vec!["Root", "Foo", "FooA", "FooA2"],
            vec!["Root", "Foo", "FooB"],
            vec!["Root", "Foo", "FooB", "FooB1"],
            vec!["Root", "Bar"],
        ];
        assert_eq!(iter_result(tree), expect);
    }
}
