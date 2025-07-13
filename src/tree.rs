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

    fn iter(&self) -> DepthFirstIterator<'_, C> {
        DepthFirstIterator {
            children: vec![],
            iterators: vec![],
        }
    }
}

// `Root<Self>` -> Any child node is root of itself
pub trait Child: Sized + Root<Self> {
    type Id: Hash + Clone;

    fn id(&self) -> Self::Id;
}

// MARK: Depth First Iterator
pub struct Item<'a, C: Child> {
    pub child: &'a C,
    pub path: Vec<C::Id>,
}

pub struct DepthFirstIterator<'a, C: Child> {
    children: Vec<C::Id>,
    iterators: Vec<Iter<'a, C>>,
}

impl<'a, C: Child> Iterator for DepthFirstIterator<'a, C> {
    type Item = Item<'a, C>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iterator) = self.iterators.last_mut() {
            if let Some(child) = iterator.next() {
                let item = Item {
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

    struct TR {
        children: Vec<TC>,
    }

    impl Root<TC> for TR {
        fn children(&self) -> &Vec<TC> {
            &self.children
        }
    }

    struct TC {
        id: &'static str,
        children: Vec<Self>,
    }

    fn tc(id: &'static str) -> TC {
        tcc(id, vec![])
    }

    fn tcc(id: &'static str, children: Vec<TC>) -> TC {
        TC { id, children }
    }

    impl Child for TC {
        type Id = &'static str;

        fn id(&self) -> Self::Id {
            self.id
        }
    }

    impl Root<Self> for TC {
        fn children(&self) -> &Vec<Self> {
            &self.children
        }
    }

    #[test]
    fn iterator() {
        #[rustfmt::skip]
        let root = TR {
            children: vec![
                tc("0"),
                tcc("1", vec![
                    tc("1.0"),
                    tc("1.1")
                ])
            ]
        };

        let string = root
            .iter()
            .map(|c| format!("{}", c))
            .collect::<Vec<String>>()
            .join("\n");

        println!("{}", string);

        // let tree = node("Root", vec![
        //     node("Foo", vec![
        //         node("FooA", vec![
        //             node("FooA1", vec![]),
        //             node("FooA2", vec![]),
        //         ]),
        //         node("FooB", vec![
        //             node("FooB1", vec![]),
        //         ]),
        //     ]),
        //     node("Bar", vec![]),
        // ]);
        // let display = tree
        //     .nested_iter()
        //     .map(|n| n.to_string())
        //     .collect::<Vec<String>>()
        //     .join("\n")
        //     + "\n";
        // let expectation = indoc! {"
        //     [Root]
        //     Root->[Foo]
        //     Root->Foo->[FooA]
        //     Root->Foo->FooA->[FooA1]
        //     Root->Foo->FooA->[FooA2]
        //     Root->Foo->[FooB]
        //     Root->Foo->FooB->[FooB1]
        //     Root->[Bar]
        // "};
        // assert_eq!(expectation, display);
    }

    // impl NestedIter for Node {
    //     fn children(&self) -> &Vec<Self> {
    //         &self.children
    //     }
    // }

    impl fmt::Display for super::Item<'_, TC> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for parent in self.path.iter() {
                write!(f, "{}->", parent)?;
            }
            write!(f, "[{}]", self.child.id)?;
            Ok(())
        }
    }

    // fn c(id: u64, children: Vec<SampleChild>) -> SampleChild {
    //     SampleChild { id, children }
    // }
}
