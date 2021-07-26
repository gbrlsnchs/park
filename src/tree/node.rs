use std::{
	ffi::{OsStr, OsString},
	path::PathBuf,
};

use thiserror::Error;

use crate::config::Options;

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
		/// The name of the link.
		link_name: OsString,
	},
}

impl Node {
	/// Adds a path to the node if and only if a node for that path doesn't exist yet.
	pub fn add(&mut self, path: PathBuf, opts: Option<Options>) -> AddResult {
		// Let's break the path into segments.
		let segments = path.iter().collect::<Vec<&OsStr>>();

		// Now let's isolate the first segment, which is the new node's key.
		if let Some((segment, rest)) = segments.split_first() {
			let segment = *segment;

			match self {
				Self::Root(children) | Self::Branch { children, .. } => {
					// Let's check whether there's already a node with same path, otherwise let's
					// just create it, if needed.
					let child = children.iter_mut().find(|node| node.get_path() == segment);
					let is_leaf = rest.is_empty();

					if is_leaf {
						let leaf_exists = child.is_some();

						if leaf_exists {
							return Err(AddError::LeafExists(segment.into()));
						}

						let link_name = if let Some(Options { link_name }) = opts {
							Some(link_name)
						} else {
							None
						};

						children.push(Self::Leaf {
							path: segment.into(),
							link_name: link_name.unwrap_or(segment.into()),
						})
					} else {
						let rest = rest.iter().collect();
						if let Some(branch) = child {
							branch.add(rest, opts)?;
						} else {
							let mut branch = Node::Branch {
								path: segment.into(),
								children: Vec::new(),
							};
							branch.add(rest, opts)?;
							children.push(branch);
						}
					}
				}
				// We only support adding new nodes to nodes that are not leaves!
				_ => return Err(AddError::LeafAsBranch(segment.into())),
			};
		}

		Ok(())
	}

	/// Returns the segment path for the node. Root panics.
	// TODO(gbrlsnchs): Add unit tests.
	fn get_path(&self) -> PathBuf {
		match self {
			Self::Leaf { path, .. } => path.into(),
			Self::Branch { path, .. } => path.into(),
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
		struct Test<'a> {
			description: &'a str,
			node_before: Node,
			input: (PathBuf, Option<Options>),
			node_after: Node,
			want: AddResult,
		}

		let test_cases = vec![
			Test {
				description: "simple first node",
				node_before: Node::Root(Vec::new()),
				input: (PathBuf::from("foo"), None),
				node_after: Node::Root(vec![Node::Leaf {
					link_name: OsString::from("foo"),
					path: PathBuf::from("foo"),
				}]),
				want: Ok(()),
			},
			Test {
				description: "simple nested node",
				node_before: Node::Root(Vec::new()),
				input: (PathBuf::from("foo/bar"), None),
				node_after: Node::Root(vec![Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::Leaf {
						link_name: OsString::from("bar"),
						path: PathBuf::from("bar"),
					}],
				}]),
				want: Ok(()),
			},
			Test {
				description: "leaf exists for simple node",
				node_before: Node::Root(vec![Node::Leaf {
					link_name: OsString::from("foo"),
					path: PathBuf::from("foo"),
				}]),
				input: (PathBuf::from("foo"), None),
				node_after: Node::Root(vec![Node::Leaf {
					link_name: OsString::from("foo"),
					path: PathBuf::from("foo"),
				}]),
				want: Err(AddError::LeafExists(PathBuf::from("foo"))),
			},
			Test {
				description: "leaf exists for nested node",
				node_before: Node::Root(vec![Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::Leaf {
						link_name: OsString::from("bar"),
						path: PathBuf::from("bar"),
					}],
				}]),
				input: (PathBuf::from("foo"), None),
				node_after: Node::Root(vec![Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::Leaf {
						link_name: OsString::from("bar"),
						path: PathBuf::from("bar"),
					}],
				}]),
				want: Err(AddError::LeafExists(PathBuf::from("foo"))),
			},
			Test {
				description: "new link name for simple first node",
				node_before: Node::Root(Vec::new()),
				input: (
					PathBuf::from("foo"),
					Some(Options {
						link_name: OsString::from("new_name"),
					}),
				),
				node_after: Node::Root(vec![Node::Leaf {
					link_name: OsString::from("new_name"),
					path: PathBuf::from("foo"),
				}]),
				want: Ok(()),
			},
			Test {
				description: "new link name for nested node",
				node_before: Node::Root(Vec::new()),
				input: (
					PathBuf::from("foo/bar"),
					Some(Options {
						link_name: OsString::from("new_name"),
					}),
				),
				node_after: Node::Root(vec![Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::Leaf {
						link_name: OsString::from("new_name"),
						path: PathBuf::from("bar"),
					}],
				}]),
				want: Ok(()),
			},
		];

		for mut case in test_cases.into_iter() {
			let got = case.node_before.add(case.input.0, case.input.1);

			assert_eq!(got, case.want, "bad result for {:?}", case.description);
			assert_eq!(
				case.node_before, case.node_after,
				"nodes mismatch for {:?}",
				case.description
			);
		}
	}

	#[test]
	fn test_error_messages() {
		let test_cases = vec![
			(
				AddError::LeafAsBranch(PathBuf::from("foo/bar")),
				r#"node for "foo/bar" is leaf, not branch"#,
			),
			(
				AddError::LeafExists(PathBuf::from("foo/bar")),
				r#"leaf already exists for "foo/bar""#,
			),
		];

		for case in test_cases.iter() {
			let got = case.0.to_string();

			assert_eq!(got, case.1);
		}
	}
}
