use std::{
	cell::RefCell,
	ffi::OsStr,
	path::{Path, PathBuf},
	rc::Rc,
};

use thiserror::Error;

use crate::config::Link;

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

/// Possible states a link node can be in.
#[derive(Debug, PartialEq)]
pub enum Status {
	/// Unknown state, probably because the node wasn't analyzed.
	Unknown,
	/// The target can by symlinked without any conflicts.
	Ready,
	/// The target is already symlinked accordingly.
	Done,
	/// Link exists, but points to a different target.
	Mismatch,
	/// Another file already exists in the link path.
	Conflict,
	/// The file supposed to serve as the link directory is not a directory.
	Obstructed,
}

pub type NodeRef = Rc<RefCell<Node>>;

/// A segment of path in Park's tree structure.
#[derive(Debug, PartialEq)]
pub enum Node {
	/// The root of the tree, has no paths, only nodes.
	Root(Vec<NodeRef>),
	/// Nodes that are branches simply hold other nodes and are never used as targets.
	Branch {
		/// Segment path for the branch.
		path: PathBuf,
		/// Nodes under the branch. May be leaves or other branches.
		children: Vec<NodeRef>,
	},
	/// These nodes are used as targets. They can't become branches.
	Leaf {
		/// Segment path for the leaf.
		target_path: PathBuf,
		/// The base directory of the link.
		link_path: PathBuf,
		/// Status of the link.
		status: Status,
	},
}

impl Node {
	/// Helper to return a node inside a RefCell inside an Rc.
	pub fn new_ref(node: Node) -> NodeRef {
		Rc::new(RefCell::new(node))
	}

	/// Adds a path to the node if and only if a node for that path doesn't exist yet.
	pub fn add(&mut self, default_base_dir: &Path, path: PathBuf, link: Link) -> AddResult {
		// Let's break the path into segments.
		let segments = path.iter().collect::<Vec<&OsStr>>();

		// Now let's isolate the first segment, which is the new node's key.
		if let Some((segment, rest)) = segments.split_first() {
			let segment = *segment;

			if let Self::Leaf { .. } = self {
				return Err(AddError::LeafAsBranch(segment.into()));
			}

			let target_path;
			let children;
			match self {
				Self::Root(root_children) => {
					children = root_children;
					target_path = PathBuf::new();
				}
				Self::Branch {
					children: branch_children,
					path,
				} => {
					children = branch_children;
					target_path = path.to_path_buf();
				}
				_ => unreachable!(),
			};

			// Let's check whether there's already a node with same path, otherwise let's
			// just create it, if needed.
			let child = children.iter().find(|node_ref| {
				let node = node_ref.borrow();

				node.get_path()
					.file_name()
					.map_or(false, |file_name| file_name == segment)
			});
			let is_leaf = rest.is_empty();

			if is_leaf {
				let leaf_exists = child.is_some();

				if leaf_exists {
					return Err(AddError::LeafExists(segment.into()));
				}

				let Link {
					base_dir,
					name: link_name,
				} = link;

				let base_dir = base_dir.unwrap_or_else(|| default_base_dir.into());
				let link_name = link_name
					.filter(|link_name| !link_name.as_os_str().is_empty())
					.unwrap_or_else(|| segment.into());

				children.push(Self::new_ref(Self::Leaf {
					target_path: target_path.join(segment),
					link_path: base_dir.join(link_name),
					status: Status::Unknown,
				}));
			} else {
				let rest = rest.iter().collect();

				if let Some(branch_ref) = child {
					let mut branch = branch_ref.borrow_mut();
					branch.add(default_base_dir, rest, link)?;
				} else {
					let mut branch = Node::Branch {
						path: segment.into(),
						children: Vec::new(),
					};

					branch.add(default_base_dir, rest, link)?;
					children.push(Rc::new(RefCell::new(branch)));
				}
			}
		}

		Ok(())
	}

	/// Returns the segment path for the node. Root panics.
	// TODO(gbrlsnchs): Add unit tests.
	fn get_path(&self) -> &Path {
		match self {
			Self::Leaf {
				target_path: path, ..
			}
			| Self::Branch { path, .. } => path,
			_ => unreachable!(),
		}
	}

	pub fn get_children(&self) -> Option<&Vec<NodeRef>> {
		match self {
			Node::Root(children) | Node::Branch { children, .. } => Some(children),
			_ => None,
		}
	}
}

#[cfg(test)]
mod tests {
	use pretty_assertions::assert_eq;

	use super::*;

	#[test]
	fn test_add() {
		let default_base_dir = PathBuf::from("default_base_dir");

		struct Test<'a> {
			description: &'a str,
			node_before: Node,
			input: (PathBuf, Link),
			node_after: Node,
			want: AddResult,
		}

		let test_cases = vec![
			Test {
				description: "simple first node",
				node_before: Node::Root(Vec::new()),
				input: (PathBuf::from("foo"), Link::default()),
				node_after: Node::Root(vec![Node::new_ref(Node::Leaf {
					link_path: default_base_dir.join("foo"),
					target_path: PathBuf::from("foo"),
					status: Status::Unknown,
				})]),
				want: Ok(()),
			},
			Test {
				description: "simple nested node",
				node_before: Node::Root(Vec::new()),
				input: (PathBuf::from("foo/bar"), Link::default()),
				node_after: Node::Root(vec![Node::new_ref(Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: default_base_dir.join("bar"),
						target_path: PathBuf::from("foo/bar"),
						status: Status::Unknown,
					})],
				})]),
				want: Ok(()),
			},
			Test {
				description: "simple node to existing branch",
				node_before: Node::Root(vec![Node::new_ref(Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: default_base_dir.join("bar"),
						target_path: PathBuf::from("foo/bar"),
						status: Status::Unknown,
					})],
				})]),
				input: (PathBuf::from("foo/test"), Link::default()),
				node_after: Node::Root(vec![Node::new_ref(Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![
						Node::new_ref(Node::Leaf {
							link_path: default_base_dir.join("bar"),
							target_path: PathBuf::from("foo/bar"),
							status: Status::Unknown,
						}),
						Node::new_ref(Node::Leaf {
							link_path: default_base_dir.join("test"),
							target_path: PathBuf::from("foo/test"),
							status: Status::Unknown,
						}),
					],
				})]),
				want: Ok(()),
			},
			Test {
				description: "leaf exists for simple node",
				node_before: Node::Root(vec![Node::new_ref(Node::Leaf {
					link_path: default_base_dir.join("foo"),
					target_path: PathBuf::from("foo"),
					status: Status::Unknown,
				})]),
				input: (PathBuf::from("foo"), Link::default()),
				node_after: Node::Root(vec![Node::new_ref(Node::Leaf {
					link_path: default_base_dir.join("foo"),
					target_path: PathBuf::from("foo"),
					status: Status::Unknown,
				})]),
				want: Err(AddError::LeafExists(PathBuf::from("foo"))),
			},
			Test {
				description: "leaf exists for nested node",
				node_before: Node::Root(vec![Node::new_ref(Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: default_base_dir.join("bar"),
						target_path: PathBuf::from("bar"),
						status: Status::Unknown,
					})],
				})]),
				input: (PathBuf::from("foo"), Link::default()),
				node_after: Node::Root(vec![Node::new_ref(Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: default_base_dir.join("bar"),
						target_path: PathBuf::from("bar"),
						status: Status::Unknown,
					})],
				})]),
				want: Err(AddError::LeafExists(PathBuf::from("foo"))),
			},
			Test {
				description: "new link name for simple first node",
				node_before: Node::Root(Vec::new()),
				input: (
					PathBuf::from("foo"),
					Link {
						name: Some(PathBuf::from("new_name")),
						..Link::default()
					},
				),
				node_after: Node::Root(vec![Node::new_ref(Node::Leaf {
					link_path: default_base_dir.join("new_name"),
					target_path: PathBuf::from("foo"),
					status: Status::Unknown,
				})]),
				want: Ok(()),
			},
			Test {
				description: "new link name for nested node",
				node_before: Node::Root(Vec::new()),
				input: (
					PathBuf::from("foo/bar"),
					Link {
						name: Some(PathBuf::from("new_name")),
						..Link::default()
					},
				),
				node_after: Node::Root(vec![Node::new_ref(Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: default_base_dir.join("new_name"),
						target_path: PathBuf::from("foo/bar"),
						status: Status::Unknown,
					})],
				})]),
				want: Ok(()),
			},
			Test {
				description: "empty link name for simple first node",
				node_before: Node::Root(Vec::new()),
				input: (
					PathBuf::from("foo"),
					Link {
						name: Some(PathBuf::new()),
						..Link::default()
					},
				),
				node_after: Node::Root(vec![Node::new_ref(Node::Leaf {
					link_path: default_base_dir.join("foo"),
					target_path: PathBuf::from("foo"),
					status: Status::Unknown,
				})]),
				want: Ok(()),
			},
			Test {
				description: "empty link name for nested node",
				node_before: Node::Root(Vec::new()),
				input: (
					PathBuf::from("foo/bar"),
					Link {
						name: Some(PathBuf::new()),
						..Link::default()
					},
				),
				node_after: Node::Root(vec![Node::new_ref(Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: default_base_dir.join("bar"),
						target_path: PathBuf::from("foo/bar"),
						status: Status::Unknown,
					})],
				})]),
				want: Ok(()),
			},
			Test {
				description: "different base directory for simple first node",
				node_before: Node::Root(Vec::new()),
				input: (
					PathBuf::from("foo"),
					Link {
						base_dir: Some(PathBuf::from("alt_base_dir")),
						..Link::default()
					},
				),
				node_after: Node::Root(vec![Node::new_ref(Node::Leaf {
					link_path: PathBuf::from("alt_base_dir").join("foo"),
					target_path: PathBuf::from("foo"),
					status: Status::Unknown,
				})]),
				want: Ok(()),
			},
			Test {
				description: "empty link name for nested node with alternative base directory",
				node_before: Node::Root(Vec::new()),
				input: (
					PathBuf::from("foo/bar"),
					Link {
						base_dir: Some(PathBuf::from("alt_base_dir")),
						..Link::default()
					},
				),
				node_after: Node::Root(vec![Node::new_ref(Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("alt_base_dir").join("bar"),
						target_path: PathBuf::from("foo/bar"),
						status: Status::Unknown,
					})],
				})]),
				want: Ok(()),
			},
		];

		for mut case in test_cases {
			let got = case
				.node_before
				.add(&default_base_dir, case.input.0, case.input.1);

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

		for case in test_cases {
			let got = case.0.to_string();

			assert_eq!(got, case.1);
		}
	}
}
