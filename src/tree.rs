/// # Two type tree
///
/// ## Implmenentation
///
/// - File is group root
/// - Group is group child
/// - Group is task root
/// - Task is task child
///
/// ```rust
/// let a = 5;
/// ```
use std::hash::Hash;
use std::slice::Iter;

/// Root of the tree structure parametrized by the type of children it contains
pub trait Parent<C: Child>: Sized {
    fn children(&self) -> &Vec<C>;

    fn children_mut(&mut self) -> &mut Vec<C>;

    fn into_children(self) -> Vec<C>;

    // Merges contents (excluding children) with self
    // The default impl is empty
    fn move_data_from(&mut self, _: &mut Self) {}

    /// Finds a child, given it's id in O(n) time
    fn child_mut(&mut self, id: C::Id) -> Option<&mut C> {
        self.children_mut().iter_mut().find(|c| c.id() == id)
    }

    /// Depth first iterator, which includes full path to the `Child`
    fn iter(&self) -> DepthFirstIterator<'_, C> {
        DepthFirstIterator {
            parent_path: vec![],
            iterators: vec![self.children().iter()],
        }
    }

    fn insert(&mut self, path: &[C::Id], child: C) -> &mut C {
        if let Some((parent_id, rest)) = path.split_first() {
            // Add parent if it does not exist
            if self.child_mut(parent_id.clone()).is_none() {
                self.children_mut().push(C::new(parent_id.clone()));
            }
            // Get a reference to the parent, which now must exist
            self.child_mut(parent_id.clone())
                .expect("Parent was inserted")
                .insert(rest, child)
        } else {
            let id = child.id();
            if let Some(existing) = self.child_mut(child.id()) {
                // Make child mutable
                let mut child = child;
                existing.move_data_from(&mut child);
                for grand_child in child.into_children() {
                    existing.insert(&[], grand_child);
                }
            } else {
                self.children_mut().push(child);
            }
            self.child_mut(id).expect("Child was inserted")
        }
    }

    /// Returns children of the last node at given level
    fn last_children(&mut self, level: u8) -> Option<&mut Vec<C>> {
        if level > 0 {
            self.children_mut()
                .last_mut()
                .map(|c| c.last_children(level - 1))?
        } else {
            Some(self.children_mut())
        }
    }

    fn for_each_mut<A>(&mut self, action: &mut A)
    where
        A: FnMut(&mut C, &Vec<C::Id>),
    {
        self.for_each_mut_internal(&mut vec![], action);
    }

    #[doc(hidden)]
    fn for_each_mut_internal<A>(&mut self, path: &mut Vec<C::Id>, action: &mut A)
    where
        A: FnMut(&mut C, &Vec<C::Id>),
    {
        self.children_mut().iter_mut().for_each(|child| {
            action(child, &path);
            path.push(child.id());
            child.for_each_mut_internal(path, action);
            path.pop();
        })
    }

    fn extract_if<P, A>(&mut self, path: &mut Vec<C::Id>, action: &mut A, filter: &P)
    where
        A: FnMut(C, &Vec<C::Id>),
        P: Fn(&mut C) -> bool,
    {
        self.children_mut()
            .extract_if(.., |c| filter(c))
            .for_each(|c| action(c, path));
        for child in self.children_mut() {
            path.push(child.id());
            child.extract_if(path, action, filter);
        }
    }
}

/// A child node that is identifyable with respect to it's parent
/// Any child node is root of it's own children
pub trait Child: Sized + Parent<Self> {
    // Identifies a child with respect to it's parent
    type Id: Hash + Clone + PartialEq;

    fn id(&self) -> Self::Id;

    fn new(id: Self::Id) -> Self;
}

// MARK: Depth First Iterator

pub struct IteratorItem<'a, C: Child> {
    pub child: &'a C,
    pub parent_path: Vec<C::Id>,
}

pub struct DepthFirstIterator<'a, C: Child> {
    parent_path: Vec<C::Id>,
    iterators: Vec<Iter<'a, C>>,
}

impl<'a, C: Child> Iterator for DepthFirstIterator<'a, C> {
    type Item = IteratorItem<'a, C>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iterator) = self.iterators.last_mut() {
            if let Some(child) = iterator.next() {
                let item = IteratorItem {
                    child,
                    parent_path: self.parent_path.clone(),
                };
                self.parent_path.push(child.id());
                self.iterators.push(child.children().iter());
                Some(item)
            } else {
                self.parent_path.pop();
                self.iterators.pop();
                self.next() // Skip while backtracking
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    #[test]
    #[rustfmt::skip]
    fn insert() {
        let mut root = TestRoot {
            children: vec![
                tc("a"),
                tcc(
                    "b",
                    vec![
                        tc("b1"),
                        tc("b2"),
                    ],
                ),
            ],
        };
        root.insert(&["c"], tc("c1"));
        root.insert(&["b"], tc("b3"));
        let expectation = TestRoot {
            children: vec![
                tc("a"),
                tcc("b", vec![
                    tc("b1"),
                    tc("b2"),
                    tc("b3")
                ]),
                tcc("c", vec![
                    tc("c1")
                ])
            ],
        };
        dbg!(&root);
        assert_eq!(root,expectation);
    }

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

    // MARK: Test types
    #[derive(Debug, PartialEq)]
    struct TestRoot {
        children: Vec<TestChild>,
    }

    impl Parent<TestChild> for TestRoot {
        fn children(&self) -> &Vec<TestChild> {
            &self.children
        }

        fn children_mut(&mut self) -> &mut Vec<TestChild> {
            &mut self.children
        }

        fn into_children(self) -> Vec<TestChild> {
            self.children
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

        fn new(id: Self::Id) -> Self {
            Self {
                id,
                children: vec![],
            }
        }
    }

    impl Parent<Self> for TestChild {
        fn children_mut(&mut self) -> &mut Vec<Self> {
            &mut self.children
        }

        fn children(&self) -> &Vec<TestChild> {
            &self.children
        }

        fn into_children(self) -> Vec<Self> {
            self.children
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

    impl fmt::Display for super::IteratorItem<'_, TestChild> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for parent in self.parent_path.iter() {
                write!(f, "{}->", parent)?;
            }
            write!(f, "[{}]", self.child.id)?;
            Ok(())
        }
    }
}
