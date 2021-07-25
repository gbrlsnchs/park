use std::{ffi::OsStr, path::PathBuf};

use thiserror::Error;

/// Possible errors when adding paths to nodes.
#[derive(Debug, Error, PartialEq)]
pub enum AddError {
	/// This represents an error for when trying to add a path to a leaf node. Only branch nodes
	/// can have paths added.
	#[error("node for {0:?} is leaf, not branch")]
	LeafAsBranch(PathBuf),
	/// This represents an error for when trying to add a node and there's already a leaf node for
	/// that same path.
	#[error("leaf already exists for {0:?}")]
	LeafExists(PathBuf),
}

/// Alias for the result when adding to a node.
pub type AddResult = Result<(), AddError>;

/// A segment of path in Park's tree structure.
#[derive(Debug, PartialEq)]
pub enum Node {
	/// The root of the tree, has no paths, only nodes.
	Root(Vec<Node>),
	/// Nodes that are branches simply hold other nodes and are never used as targets.
	Branch {
		/// Segment path for the branch.
		path: PathBuf,
		/// Nodes under the branch. May be leaves or other branches.
		children: Vec<Node>,
	},
	/// These nodes are used as targets. They can't become branches.
	Leaf {
		/// Segment path for the leaf.
		path: PathBuf,
	},
}

impl Node {
	/// Adds a path to the node if and only if a node for that path doesn't exist yet.
	pub fn add(&mut self, path: PathBuf) -> AddResult {
		// Let's break the path into segments.
		let segments = path.iter().collect::<Vec<&OsStr>>();

		// Now let's isolate the first segment, which is the new node's key.
		if let Some((segment, rest)) = segments.split_first() {
			let segment = *segment;

			// We only support adding new nodes to nodes that are not leaves!
			match self {
				Self::Root(children) | Self::Branch { children, .. } => {
					// Let's check whether there's already a node with same name, otherwise let's
					// just create it.
					let child = children.iter_mut().find(|node| node.get_path() == segment);
					let is_leaf = rest.is_empty();

					if is_leaf {
						let leaf_exists = child.is_some();

						if leaf_exists {
							return Err(AddError::LeafExists(segment.into()));
						}

						children.push(Self::Leaf {
							path: segment.into(),
						})
					} else {
						let rest = rest.iter().collect();
						if let Some(branch) = child {
							branch.add(rest)?;
						} else {
							let mut branch = Node::Branch {
								path: segment.into(),
								children: Vec::new(),
							};
							branch.add(rest)?;
							children.push(branch);
						}
					}
				}
				_ => return Err(AddError::LeafAsBranch(segment.into())),
			};
		}

		Ok(())
	}

	/// Returns the segment path for the node. Root panics.
	// TODO(gbrlsnchs): Add unit tests.
	fn get_path(&self) -> PathBuf {
		match self {
			Self::Leaf { path } => path.into(),
			Self::Branch { path, children: _ } => path.into(),
			_ => panic!("Can't get path for root node."),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	use pretty_assertions::assert_eq;

	#[test]
	fn test_add() {
		struct Test {
			node_before: Node,
			input: PathBuf,
			node_after: Node,
			want: AddResult,
		}

		let test_cases = vec![
			Test {
				node_before: Node::Root(Vec::new()),
				input: PathBuf::from("foo"),
				node_after: Node::Root(vec![Node::Leaf {
					path: PathBuf::from("foo"),
				}]),
				want: Ok(()),
			},
			Test {
				node_before: Node::Root(Vec::new()),
				input: PathBuf::from("foo/bar"),
				node_after: Node::Root(vec![Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::Leaf {
						path: PathBuf::from("bar"),
					}],
				}]),
				want: Ok(()),
			},
			Test {
				node_before: Node::Root(vec![Node::Leaf {
					path: PathBuf::from("foo"),
				}]),
				input: PathBuf::from("foo"),
				node_after: Node::Root(vec![Node::Leaf {
					path: PathBuf::from("foo"),
				}]),
				want: Err(AddError::LeafExists(PathBuf::from("foo"))),
			},
			Test {
				node_before: Node::Root(vec![Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::Leaf {
						path: PathBuf::from("bar"),
					}],
				}]),
				input: PathBuf::from("foo"),
				node_after: Node::Root(vec![Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::Leaf {
						path: PathBuf::from("bar"),
					}],
				}]),
				want: Err(AddError::LeafExists(PathBuf::from("foo"))),
			},
		];

		for mut case in test_cases.into_iter() {
			let got = case.node_before.add(case.input);

			assert_eq!(got, case.want);
			assert_eq!(case.node_before, case.node_after);
		}
	}
}
