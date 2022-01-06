use std::rc::Rc;

use super::node::{Node, NodeRef};

pub struct DepthFirstIter {
	stack: Vec<NodeRef>,
}

impl DepthFirstIter {
	pub fn new(root: Node) -> Self {
		DepthFirstIter {
			stack: vec![Node::new_ref(root)],
		}
	}
}

impl<'a> Iterator for DepthFirstIter {
	type Item = NodeRef;

	fn next(&mut self) -> Option<Self::Item> {
		let current = self.stack.pop()?;
		let node = current.borrow();
		let children = node.get_children();

		for child in children.iter().flatten().rev() {
			self.stack.push(Rc::clone(child));
		}

		Some(Rc::clone(&current))
	}
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use pretty_assertions::assert_eq;

	use crate::tree::node::Status;

	use super::*;

	#[test]
	fn depth_first_iterator() {
		let root = Node::Root(vec![
			Node::new_ref(Node::Branch {
				path: PathBuf::from("foo"),
				children: vec![Node::new_ref(Node::Leaf {
					link_path: PathBuf::new().join("bar"),
					target_path: PathBuf::from("bar"),
					status: Status::Unknown,
				})],
			}),
			Node::new_ref(Node::Branch {
				path: PathBuf::from("qux"),
				children: vec![Node::new_ref(Node::Leaf {
					link_path: PathBuf::new().join("quux"),
					target_path: PathBuf::from("quux"),
					status: Status::Unknown,
				})],
			}),
		]);

		let iter = DepthFirstIter::new(root);
		let got = iter.collect::<Vec<NodeRef>>();

		assert_eq!(
			got,
			vec![
				Node::new_ref(Node::Root(vec![
					Node::new_ref(Node::Branch {
						path: PathBuf::from("foo"),
						children: vec![Node::new_ref(Node::Leaf {
							link_path: PathBuf::new().join("bar"),
							target_path: PathBuf::from("bar"),
							status: Status::Unknown,
						})],
					}),
					Node::new_ref(Node::Branch {
						path: PathBuf::from("qux"),
						children: vec![Node::new_ref(Node::Leaf {
							link_path: PathBuf::new().join("quux"),
							target_path: PathBuf::from("quux"),
							status: Status::Unknown,
						})],
					}),
				])),
				Node::new_ref(Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::new().join("bar"),
						target_path: PathBuf::from("bar"),
						status: Status::Unknown,
					})],
				}),
				Node::new_ref(Node::Leaf {
					link_path: PathBuf::new().join("bar"),
					target_path: PathBuf::from("bar"),
					status: Status::Unknown,
				}),
				Node::new_ref(Node::Branch {
					path: PathBuf::from("qux"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::new().join("quux"),
						target_path: PathBuf::from("quux"),
						status: Status::Unknown,
					})],
				}),
				Node::new_ref(Node::Leaf {
					link_path: PathBuf::new().join("quux"),
					target_path: PathBuf::from("quux"),
					status: Status::Unknown,
				}),
			]
		);
	}
}
