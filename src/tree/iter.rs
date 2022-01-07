use std::rc::Rc;

use super::node::NodeRef;

#[derive(Debug, PartialEq)]
pub struct NodeEntry {
	/// Whether the node is the deepest node in its level.
	pub deepest: bool,
	/// Level of the node. Root is at level 0, its children are at level 1, and so on.
	pub level: usize,
	/// Managed reference to the node it points to. Can be borrowed, mutably or not.
	pub node_ref: NodeRef,
}

/// Iterator for Tree. Performs depth-first search iteration.
pub struct DepthFirstIter {
	/// Stack that holds node entries.
	stack: Vec<NodeEntry>,
}

impl DepthFirstIter {
	pub fn new(root_ref: NodeRef) -> Self {
		DepthFirstIter {
			stack: vec![NodeEntry {
				deepest: false,
				level: 0,
				node_ref: root_ref,
			}],
		}
	}
}

impl<'a> Iterator for DepthFirstIter {
	type Item = NodeEntry;

	fn next(&mut self) -> Option<Self::Item> {
		let current = self.stack.pop()?;
		let node = current.node_ref.borrow();
		let children = node.get_children();

		for (idx, child) in children.iter().flatten().rev().enumerate() {
			self.stack.push(NodeEntry {
				deepest: idx == 0,
				level: current.level + 1,
				node_ref: Rc::clone(child),
			});
		}

		Some(NodeEntry {
			deepest: current.deepest,
			level: current.level,
			node_ref: Rc::clone(&current.node_ref),
		})
	}
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use pretty_assertions::assert_eq;

	use crate::tree::node::{Node, Status};

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

		let iter = DepthFirstIter::new(Node::new_ref(root));
		let got = iter.collect::<Vec<NodeEntry>>();

		assert_eq!(
			got,
			vec![
				NodeEntry {
					deepest: false,
					level: 0,
					node_ref: Node::new_ref(Node::Root(vec![
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
					]))
				},
				NodeEntry {
					deepest: false,
					level: 1,
					node_ref: Node::new_ref(Node::Branch {
						path: PathBuf::from("foo"),
						children: vec![Node::new_ref(Node::Leaf {
							link_path: PathBuf::new().join("bar"),
							target_path: PathBuf::from("bar"),
							status: Status::Unknown,
						})],
					})
				},
				NodeEntry {
					level: 2,
					deepest: true,
					node_ref: Node::new_ref(Node::Leaf {
						link_path: PathBuf::new().join("bar"),
						target_path: PathBuf::from("bar"),
						status: Status::Unknown,
					})
				},
				NodeEntry {
					deepest: true,
					level: 1,
					node_ref: Node::new_ref(Node::Branch {
						path: PathBuf::from("qux"),
						children: vec![Node::new_ref(Node::Leaf {
							link_path: PathBuf::new().join("quux"),
							target_path: PathBuf::from("quux"),
							status: Status::Unknown,
						})],
					})
				},
				NodeEntry {
					deepest: true,
					level: 2,
					node_ref: Node::new_ref(Node::Leaf {
						link_path: PathBuf::new().join("quux"),
						target_path: PathBuf::from("quux"),
						status: Status::Unknown,
					})
				},
			]
		);
	}
}
