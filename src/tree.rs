// Two type tree
//
// - File is group root
// - Group is group child
// - Group is task root
// - Task is task child

use std::hash::Hash;
use std::slice::Iter;

// Only one root ever present - no need to be identified
pub trait Root<C: Child>: Sized {
    fn children(&self) -> &Vec<C>;

    fn nested_iter(&self) -> DepthFirstIterator<'_, C> {
        DepthFirstIterator {
            children: vec![],
            iterators: vec![],
        }
    }
}

// Any child node is root of itself
pub trait Child: Sized + Root<Self> {
    type Id: Hash + Clone;

    fn id(&self) -> Self::Id;
}

// MARK: Depth First Iterator

pub struct IteratorItem<'a, C: Child> {
    pub child: &'a C,
    pub path: Vec<C::Id>,
}

pub struct DepthFirstIterator<'a, C: Child> {
    children: Vec<C::Id>,
    iterators: Vec<Iter<'a, C>>,
}

impl<'a, C: Child> Iterator for DepthFirstIterator<'a, C> {
    type Item = IteratorItem<'a, C>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iterator) = self.iterators.last_mut() {
            if let Some(child) = iterator.next() {
                let item = IteratorItem {
                    child,
                    path: self.children.clone(),
                };
                self.children.push(child.id());
                self.iterators.push(child.children().iter());
                Some(item)
            } else {
                self.children.pop();
                self.iterators.pop();
                self.next() // Recursive call while backtracking
            }
        } else {
            None
        }
    }
}

// MARK: Index

mod tests {
    use super::*;
    use indoc::indoc;
    use std::fmt;

    struct SampleRoot {
        children: Vec<SampleChild>,
    }

    impl Root<SampleChild> for SampleRoot {
        fn children(&self) -> &Vec<SampleChild> {
            &self.children
        }
    }

    struct SampleChild {
        id: u64,
        children: Vec<Self>,
    }

    impl Child for SampleChild {
        type Id = u64;

        fn id(&self) -> Self::Id {
            0
        }
    }

    impl Root<Self> for SampleChild {
        fn children(&self) -> &Vec<Self> {
            &self.children
        }
    }

    // #[test]
    // fn iterator() {
    //     #[rustfmt::skip]
    //     let tree = node("Root", vec![
    //         node("Foo", vec![
    //             node("FooA", vec![
    //                 node("FooA1", vec![]),
    //                 node("FooA2", vec![]),
    //             ]),
    //             node("FooB", vec![
    //                 node("FooB1", vec![]),
    //             ]),
    //         ]),
    //         node("Bar", vec![]),
    //     ]);
    //     let display = tree
    //         .nested_iter()
    //         .map(|n| n.to_string())
    //         .collect::<Vec<String>>()
    //         .join("\n")
    //         + "\n";
    //     let expectation = indoc! {"
    //         [Root]
    //         Root->[Foo]
    //         Root->Foo->[FooA]
    //         Root->Foo->FooA->[FooA1]
    //         Root->Foo->FooA->[FooA2]
    //         Root->Foo->[FooB]
    //         Root->Foo->FooB->[FooB1]
    //         Root->[Bar]
    //     "};
    //     assert_eq!(expectation, display);
    // }

    // impl NestedIter for Node {
    //     fn children(&self) -> &Vec<Self> {
    //         &self.children
    //     }
    // }

    // impl fmt::Display for super::Path<'_, Node> {
    //     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    //         for parent in self.parents.iter() {
    //             write!(f, "{}->", parent.text)?;
    //         }
    //         write!(f, "[{}]", self.leaf.text)?;
    //         Ok(())
    //     }
    // }

    // fn node(text: &str, children: Vec<Node>) -> Node {
    //     Node {
    //         text: text.to_string(),
    //         children,
    //     }
    // }
}
