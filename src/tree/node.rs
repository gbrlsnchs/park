use std::{ffi::OsStr, path::PathBuf};

use thiserror::Error;

/// Possible errors when adding nodes.
#[derive(Debug, Error, PartialEq)]
pub enum AddError {
	#[error("node for {0:?} is leaf, not branch")]
	LeafAsBranch(PathBuf),
	#[error("leaf already exists for {0:?}")]
	LeafExists(PathBuf),
}

pub type AddResult = Result<(), AddError>;

#[derive(Debug, PartialEq)]
pub enum Node {
	Root(Vec<Node>),
	Branch { path: PathBuf, children: Vec<Node> },
	Leaf { path: PathBuf },
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

					if rest.is_empty() {
						if child.is_some() {
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
