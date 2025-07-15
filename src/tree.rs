// # Two type tree
//
// ## Implmenentation
//
// - File is group root
// - Group is group child
// - Group is task root
// - Task is task child

use std::hash::Hash;
use std::slice::Iter;

/// Root of the tree structure parametrized by the type of children it contains
pub trait Parent<C: Child>: Sized {
    // Requirements

    fn children(&self) -> &Vec<C>;

    fn children_mut(&mut self) -> &mut Vec<C>;

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

    /// Inserts node, given a node to a parent
    fn insert(&mut self, path: &[C::Id], insert_child: C) {
        if let Some(id) = path.first() {
            let next_child = if let Some(child) = self.child_mut(id.clone()) {
                child
            } else {
                self.children_mut().push(C::new(id.clone()));
                self.children_mut().last_mut().unwrap()
            };
            next_child.insert(&path[1..], insert_child);
        } else {
            self.children_mut().push(insert_child);
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
        A: FnMut(&mut C),
    {
        self.children_mut().iter_mut().for_each(|child| {
            action(child);
            child.for_each_mut(action);
        })
    }

    fn extract_if<P, A>(&mut self, filter: &mut P, action: &mut A)
    where
        P: Fn(&mut C) -> bool,
        A: FnMut(C),
    {
        self.children_mut()
            .extract_if(.., |c| filter(c))
            .for_each(|c| action(c));
        for child in self.children_mut() {
            child.extract_if(filter, action);
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
    fn insert() {
        let mut root = test_root();
        root.insert(&["0", "HEY"], tc("child"));
        println!("{:#?}", root);
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
