use std::path::{Path, PathBuf};

use super::Node;

#[cfg(test)]
mod tests;

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
