// # Two type tree
//
// ## Implmenentation
//
// - File is group root
// - Group is group child
// - Group is task root
// - Task is task child
//
// ## Merging
//
// Interface - a merge function on root that consumes both and returns owned

use std::hash::Hash;
use std::slice::Iter;

pub trait Root<C: Child>: Sized {
    // Requirements

    fn children(&self) -> &[C];

    fn children_mut(&mut self) -> &mut Vec<C>;

    // Defautl impl

    fn iter(&self) -> DepthFirstIterator<'_, C> {
        DepthFirstIterator {
            parent_path: vec![],
            iterators: vec![self.children().iter()],
        }
    }

    // fn get(&self, path: &[C::Id]) -> Option<&C> {
    //     path.split_first().and_then(|(id, remaining_path)| {
    //         self.children()
    //             .iter()
    //             .find(|c| &c.id() == id)
    //             .and_then(|c| {
    //                 if remaining_path.is_empty() {
    //                     Some(c)
    //                 } else {
    //                     c.get(remaining_path)
    //                 }
    //             })
    //     })
    // }

    // fn get_mut(&mut self, path: &[C::Id]) -> Option<&mut C> {
    //     path.split_first().and_then(|(id, remaining_path)| {
    //         self.children_mut()
    //             .iter_mut()
    //             .find(|c| &c.id() == id)
    //             .and_then(|c| {
    //                 if remaining_path.is_empty() {
    //                     Some(c)
    //                 } else {
    //                     c.get_mut(remaining_path)
    //                 }
    //             })
    //     })
    // }

    fn merged_with(mut self, mut other: Self) -> Self {
        for other_child in other.children_mut().drain(..) {
            if let Some(pos) = self
                .children_mut()
                .iter()
                .position(|child_a| child_a.id() == other_child.id())
            {
                let a_child = self.children_mut().remove(pos);
                let merged = a_child.merged_with(other_child);
                self.children_mut().insert(pos, merged);
            } else {
                self.children_mut().push(other_child);
            }
        }
        self
    }
}

// Any child node is root of it's own children
pub trait Child: Sized + Root<Self> {
    // Identifies a child with respect to it's parent
    type Id: Hash + Clone + PartialEq;

    fn id(&self) -> Self::Id;
}

// MARK: Depth First Iterator
pub struct Item<'a, C: Child> {
    pub child: &'a C,
    pub parent_path: Vec<C::Id>,
}

pub struct DepthFirstIterator<'a, C: Child> {
    parent_path: Vec<C::Id>,
    iterators: Vec<Iter<'a, C>>,
}

impl<'a, C: Child> Iterator for DepthFirstIterator<'a, C> {
    type Item = Item<'a, C>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iterator) = self.iterators.last_mut() {
            if let Some(child) = iterator.next() {
                let item = Item {
                    child,
                    parent_path: self.parent_path.clone(),
                };
                self.parent_path.push(child.id());
                self.iterators.push(child.children().iter());
                Some(item)
            } else {
                self.parent_path.pop();
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
    use std::fmt;

    #[test]
    fn iterator() {
        let display = test_root()
            .iter()
            .map(|c| format!("{}", c))
            .collect::<Vec<String>>()
            .join("\n")
            + "\n";
        let expectation = indoc::indoc! {"
            [0]
            [1]
            1->[1.0]
            1->1.0->[1.0.0]
            1->1.0->1.0.0->[1.0.0.0]
            1->[1.1]
            [2]
        "};
        assert_eq!(expectation, display);
    }

    // #[test]
    // #[rustfmt::skip]
    // fn index() {
    //     let root = test_root();
    //     assert_eq!(
    //         root.get(&[]),
    //         None
    //     );
    //     assert_eq!(
    //         root.get(&["1", "INVALID"]),
    //         None
    //     );
    //     assert_eq!(
    //         root.get(&["1", "1.1"]),
    //         Some(&tc("1.1"))
    //     );
    //     assert_eq!(
    //         root.get(&["1", "1.0", "1.0.0", "1.0.0.0"]),
    //         Some(&tc("1.0.0.0"))
    //     );
    //     assert_eq!(
    //         root.get_mut(&["1", "1.0", "1.0.0", "1.0.0.0"]),
    //         Some(&mut tc("1.0.0.0"))
    //     );
    // }

    #[test]
    #[rustfmt::skip]
    fn merge() {
        let a = test_root();
        let b = TestRoot {
            children: vec![
                tc("0"), // Check for no duplications
                tcc("1", vec![
                    tc("NESTED_NEW"), // Merging at different offset
                    tc("1.1")],
                ),
                tc("NEW") // Check merging new
            ],
        };
        let merged = a.merged_with(b);
        let expectation = TestRoot {
            children: vec![
                tc("0"),
                tcc("1", vec![
                    tcc("1.0", vec![
                        tcc("1.0.0", vec![
                            tc("1.0.0.0")
                        ])
                    ]),
                    tc("1.1"),
                    tc("NESTED_NEW") // Curently appended as last
                ]),
                tc("2"),
                tc("NEW")
            ]
        };
        assert_eq!(merged, expectation);
    }

    // MARK: Test types
    #[derive(Debug, PartialEq)]
    struct TestRoot {
        children: Vec<TestChild>,
    }

    impl Root<TestChild> for TestRoot {
        fn children(&self) -> &[TestChild] {
            &self.children
        }

        fn children_mut(&mut self) -> &mut Vec<TestChild> {
            &mut self.children
        }
    }

    #[derive(Debug, PartialEq)]
    struct TestChild {
        id: &'static str,
        children: Vec<Self>,
    }

    impl Child for TestChild {
        type Id = &'static str;

        fn id(&self) -> Self::Id {
            self.id
        }
    }

    impl Root<Self> for TestChild {
        fn children_mut(&mut self) -> &mut Vec<Self> {
            &mut self.children
        }

        fn children(&self) -> &[Self] {
            &self.children
        }
    }

    // MARK: Helpers

    #[rustfmt::skip]
    fn test_root() -> TestRoot {
        TestRoot {
            children: vec![
                tc("0"),
                tcc("1", vec![
                    tcc("1.0", vec![
                        tcc("1.0.0", vec![
                            tc("1.0.0.0")
                        ])
                    ]),
                    tc("1.1")
                ]),
                tc("2")
            ]
        }
    }

    fn tc(id: &'static str) -> TestChild {
        tcc(id, vec![])
    }

    fn tcc(id: &'static str, children: Vec<TestChild>) -> TestChild {
        TestChild { id, children }
    }

    impl fmt::Display for super::Item<'_, TestChild> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for parent in self.parent_path.iter() {
                write!(f, "{}->", parent)?;
            }
            write!(f, "[{}]", self.child.id)?;
            Ok(())
        }
    }
}
