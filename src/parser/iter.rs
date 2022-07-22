use std::path::{Path, PathBuf};

use super::node::Node;

/// Some metadata for a node inside a tree.
#[derive(Debug, PartialEq)]
pub struct NodeMetadata {
	/// Level of the node. Root is at level 0.
	pub level: usize,
	/// Whether the node is the last of its siblings.
	pub last_sibling: bool,
}

/// Iteration element. Holds all relevant data from a node.
#[derive(Debug, PartialEq)]
pub struct Element {
	pub metadata: NodeMetadata,
	pub target_path: PathBuf,
	pub link_path: Option<PathBuf>,
}

/// Iterator that visits nodes using preorder traversal.
pub struct Iter<'a> {
	stack: Vec<State<'a>>,
	path_stack: Vec<&'a Path>,
}

impl<'a> From<&'a Node> for Iter<'a> {
	fn from(root: &'a Node) -> Self {
		Iter {
			stack: vec![State {
				node: root,
				segment: None,
				metadata: NodeMetadata {
					level: 0,            // root is always at level 0
					last_sibling: false, // doesn't really matter
				},
			}],
			path_stack: Vec::new(),
		}
	}
}

impl<'a> Iterator for Iter<'a> {
	type Item = Element;

	fn next(&mut self) -> Option<Self::Item> {
		let State {
			metadata: info,
			segment,
			node,
		} = self.stack.pop()?;

		while info.level > 0 && info.level <= self.path_stack.len() {
			self.path_stack.pop();
		}

		if let Some(segment) = segment {
			self.path_stack.push(segment);
		}

		if let Some(children) = node.get_children() {
			for (idx, (segment, child)) in children.iter().rev().enumerate() {
				self.stack.push(State {
					metadata: NodeMetadata {
						level: info.level + 1,
						last_sibling: idx == 0,
					},
					segment: Some(segment),
					node: child,
				});
			}
		}

		Some(Element {
			metadata: info,
			target_path: self.path_stack.iter().collect(),
			link_path: node.get_link_path().map(PathBuf::from),
		})
	}
}

/// Iteration state.
struct State<'a> {
	metadata: NodeMetadata,
	segment: Option<&'a Path>,
	node: &'a Node,
}

#[cfg(test)]
mod tests {
	use pretty_assertions::assert_eq;

	use crate::parser::node::Edges;

	use super::*;

	#[test]
	fn iterate_in_correct_order() {
		let root = Node::Branch(Edges::from([
			(
				"baz".into(),
				Node::Branch(Edges::from([("qux".into(), Node::Leaf("test/qux".into()))])),
			),
			(
				"foo".into(),
				Node::Branch(Edges::from([("bar".into(), Node::Leaf("test/bar".into()))])),
			),
			("test".into(), Node::Leaf("something/else".into())),
		]));
		let mut iter = Iter {
			stack: Vec::from([State {
				node: &root,
				segment: None,
				metadata: NodeMetadata {
					level: 0,
					last_sibling: false,
				},
			}]),
			path_stack: Vec::new(),
		};

		assert_eq!(
			iter.next(),
			Some(Element {
				metadata: NodeMetadata {
					level: 0,
					last_sibling: false
				},
				target_path: "".into(),
				link_path: None,
			}),
		);
		assert_eq!(
			iter.next(),
			Some(Element {
				metadata: NodeMetadata {
					level: 1,
					last_sibling: false
				},
				target_path: "baz".into(),
				link_path: None,
			}),
		);
		assert_eq!(
			iter.next(),
			Some(Element {
				metadata: NodeMetadata {
					level: 2,
					last_sibling: true
				},
				target_path: "baz/qux".into(),
				link_path: Some("test/qux".into()),
			}),
		);
		assert_eq!(
			iter.next(),
			Some(Element {
				metadata: NodeMetadata {
					level: 1,
					last_sibling: false
				},
				target_path: "foo".into(),
				link_path: None,
			}),
		);
		assert_eq!(
			iter.next(),
			Some(Element {
				metadata: NodeMetadata {
					level: 2,
					last_sibling: true
				},
				target_path: "foo/bar".into(),
				link_path: Some("test/bar".into()),
			}),
		);
		assert_eq!(
			iter.next(),
			Some(Element {
				metadata: NodeMetadata {
					level: 1,
					last_sibling: true
				},
				target_path: "test".into(),
				link_path: Some("something/else".into()),
			}),
		);
		assert_eq!(iter.next(), None);
	}
}
