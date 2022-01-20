use std::path::{Path, PathBuf};

use super::node::Node;

#[derive(Debug, PartialEq)]
pub struct NodeMetadata {
	pub level: usize,
	pub last_edge: bool,
}

#[derive(Debug, PartialEq)]
pub struct NodeIterEntry {
	pub metadata: NodeMetadata,
	pub target_path: PathBuf,
	pub link_path: Option<PathBuf>,
}

struct NodeIterItem<'a> {
	metadata: NodeMetadata,
	segment: Option<&'a Path>,
	node: &'a Node,
}

pub struct DepthFirstIter<'a> {
	stack: Vec<NodeIterItem<'a>>,
	path_stack: Vec<&'a Path>,
}

impl<'a> From<&'a Node> for DepthFirstIter<'a> {
	fn from(root: &'a Node) -> Self {
		DepthFirstIter {
			stack: vec![NodeIterItem {
				node: root,
				segment: None,
				metadata: NodeMetadata {
					level: 0,
					last_edge: false,
				},
			}],
			path_stack: Vec::new(),
		}
	}
}

impl<'a> Iterator for DepthFirstIter<'a> {
	type Item = NodeIterEntry;

	fn next(&mut self) -> Option<Self::Item> {
		let NodeIterItem {
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
				self.stack.push(NodeIterItem {
					metadata: NodeMetadata {
						level: info.level + 1,
						last_edge: idx == 0,
					},
					segment: Some(segment),
					node: child,
				});
			}
		}

		Some(NodeIterEntry {
			metadata: info,
			target_path: self.path_stack.iter().collect(),
			link_path: node.get_link_path(),
		})
	}
}

#[cfg(test)]
mod tests {
	use indexmap::indexmap;
	use pretty_assertions::assert_eq;

	use super::*;

	#[test]
	fn iterate_in_correct_order() {
		let root = Node::Branch(indexmap! {
			"baz".into() => Node::Branch(indexmap!{
				"qux".into() => Node::Leaf("test/quxlinkku".into()),
			}),
			"test".into() => Node::Leaf("something/else".into()),
			"foo".into() => Node::Branch(indexmap!{
				"bar".into() => Node::Leaf("test/barlinkku".into()),
			}),
		});
		let mut iter = DepthFirstIter {
			stack: vec![NodeIterItem {
				node: &root,
				segment: None,
				metadata: NodeMetadata {
					level: 0,
					last_edge: false,
				},
			}],
			path_stack: Vec::new(),
		};

		assert_eq!(
			iter.next(),
			Some(NodeIterEntry {
				metadata: NodeMetadata {
					level: 0,
					last_edge: false
				},
				target_path: "".into(),
				link_path: None,
			}),
		);
		assert_eq!(
			iter.next(),
			Some(NodeIterEntry {
				metadata: NodeMetadata {
					level: 1,
					last_edge: false
				},
				target_path: "baz".into(),
				link_path: None,
			}),
		);
		assert_eq!(
			iter.next(),
			Some(NodeIterEntry {
				metadata: NodeMetadata {
					level: 2,
					last_edge: true
				},
				target_path: "baz/qux".into(),
				link_path: Some("test/quxlinkku".into()),
			}),
		);
		assert_eq!(
			iter.next(),
			Some(NodeIterEntry {
				metadata: NodeMetadata {
					level: 1,
					last_edge: false
				},
				target_path: "test".into(),
				link_path: Some("something/else".into()),
			}),
		);
		assert_eq!(
			iter.next(),
			Some(NodeIterEntry {
				metadata: NodeMetadata {
					level: 1,
					last_edge: true
				},
				target_path: "foo".into(),
				link_path: None,
			}),
		);
		assert_eq!(
			iter.next(),
			Some(NodeIterEntry {
				metadata: NodeMetadata {
					level: 2,
					last_edge: true
				},
				target_path: "foo/bar".into(),
				link_path: Some("test/barlinkku".into()),
			}),
		);
		assert_eq!(iter.next(), None);
	}
}
