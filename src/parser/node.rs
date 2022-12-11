use std::{
	ffi::OsStr,
	path::{Path, PathBuf},
};

use thiserror::Error;

use super::iter::{Element, Iter};

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
	/// Target can be created but the parent directory will need to be created as well.
	Unparented,
	/// Another file already exists in the link path.
	Conflict,
	/// The file supposed to serve as the link directory is not a directory.
	Obstructed,
}

#[derive(Debug, Error, PartialEq)]
pub enum Error {
	#[error("node for link {1:?} at segment {0:?} cannot be inserted because it is not a branch")]
	NotABranch(PathBuf, PathBuf),
	#[error("node for link {1:?} at segment {0:?} already exists as a leaf")]
	LeafExists(PathBuf, PathBuf),
	#[error("cannot add empty link path")]
	EmptySegment,
}

/// A vector of edges.
pub type Edges = Vec<Edge>;

/// An edge holds both its path and its respective node.
pub type Edge = (PathBuf, Node);

/// Node for a recursive tree that holds symlink paths. It is either a branch or a leaf.
#[derive(Debug, PartialEq)]
pub enum Node {
	Branch(Edges),
	Leaf(PathBuf),
}

impl Node {
	/// Adds new paths to the node. Each segment becomes a new node.
	pub fn add(&mut self, segments: Vec<&OsStr>, link_path: PathBuf) -> Result<(), Error> {
		let segments = segments.split_first();
		if segments.is_none() {
			return Err(Error::EmptySegment);
		}

		// TODO: Handle error.
		let (key, segments) = segments.unwrap();
		let key = PathBuf::from(key);

		match self {
			Self::Branch(edges) => {
				let current_slot = edges.iter_mut().find(|(path, _)| path == &key);
				let is_leaf = segments.is_empty();

				if is_leaf {
					if current_slot.is_some() {
						return Err(Error::LeafExists(key, link_path));
					}

					edges.push((key, Self::Leaf(link_path)));
				} else if let Some(edge) = current_slot {
					let (_, ref mut branch_node) = edge;

					branch_node.add(segments.into(), link_path)?;
				} else {
					let mut branch_node = Self::Branch(Edges::new());
					branch_node.add(segments.into(), link_path)?;
					edges.push((key, branch_node));
				}
				Ok(())
			}
			Self::Leaf { .. } => Err(Error::NotABranch(key, link_path)),
		}
	}

	/// Returns the node's children if it's a branch, otherwise returns None.
	pub fn get_children(&self) -> Option<&Edges> {
		match self {
			Self::Branch(edges) => Some(edges),
			Self::Leaf(..) => None,
		}
	}

	/// Returns the node's link path if it's a leaf, otherwise returns None.
	pub fn get_link_path(&self) -> Option<&Path> {
		match self {
			Self::Branch(_) => None,
			Self::Leaf(path) => Some(path),
		}
	}
}

impl<'a> IntoIterator for &'a Node {
	type Item = Element;
	type IntoIter = Iter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.into()
	}
}

#[cfg(test)]

mod tests {
	use std::ffi::OsString;

	use pretty_assertions::assert_eq;

	use super::*;

	#[test]
	fn test_add_nodes() {
		struct Test<'a> {
			description: &'a str,
			input: (Node, Vec<&'a OsStr>, PathBuf),
			output: (Node, Result<(), Error>),
		}

		let foo = OsString::from("foo");
		let bar = OsString::from("bar");
		let baz = OsString::from("baz");
		let capital_e = OsString::from("E");

		let test_cases = Vec::from([
			Test {
				description: "simple first node",
				input: (
					Node::Branch(Edges::new()),
					Vec::from([&foo[..]]),
					"test/foo".into(),
				),
				output: (
					Node::Branch(Edges::from([("foo".into(), Node::Leaf("test/foo".into()))])),
					Ok(()),
				),
			},
			Test {
				description: "add sibling node to existing one",
				input: (
					Node::Branch(Edges::from([("foo".into(), Node::Leaf("test/foo".into()))])),
					Vec::from([&bar[..]]),
					"yay/bar".into(),
				),
				output: (
					Node::Branch(Edges::from([
						("foo".into(), Node::Leaf("test/foo".into())),
						("bar".into(), Node::Leaf("yay/bar".into())),
					])),
					Ok(()),
				),
			},
			Test {
				description: "add nested node",
				input: (
					Node::Branch(Edges::new()),
					Vec::from([&foo[..], &bar[..]]),
					"test/bar".into(),
				),
				output: (
					Node::Branch(Edges::from([(
						"foo".into(),
						Node::Branch(Edges::from([("bar".into(), Node::Leaf("test/bar".into()))])),
					)])),
					Ok(()),
				),
			},
			Test {
				description: "add sibling to nested node",
				input: (
					Node::Branch(Edges::from([(
						"foo".into(),
						Node::Branch(Edges::from([("bar".into(), Node::Leaf("test/bar".into()))])),
					)])),
					Vec::from([&foo[..], &baz[..]]),
					"yay/baz".into(),
				),
				output: (
					Node::Branch(Edges::from([(
						"foo".into(),
						Node::Branch(Edges::from([
							("bar".into(), Node::Leaf("test/bar".into())),
							("baz".into(), Node::Leaf("yay/baz".into())),
						])),
					)])),
					Ok(()),
				),
			},
			Test {
				description: "add existing node path",
				input: (
					Node::Branch(Edges::from([(
						"foo".into(),
						Node::Branch(Edges::from([("bar".into(), Node::Leaf("test/bar".into()))])),
					)])),
					Vec::from([&foo[..], &bar[..]]),
					"please/let_me_in".into(),
				),
				output: (
					Node::Branch(Edges::from([(
						"foo".into(),
						Node::Branch(Edges::from([("bar".into(), Node::Leaf("test/bar".into()))])),
					)])),
					Err(Error::LeafExists("bar".into(), "please/let_me_in".into())),
				),
			},
			Test {
				description: "add node to a leaf node",
				input: (
					Node::Branch(Edges::from([("foo".into(), Node::Leaf("test/foo".into()))])),
					Vec::from([&foo[..], &bar[..]]),
					"please/let_me_in".into(),
				),
				output: (
					Node::Branch(Edges::from([("foo".into(), Node::Leaf("test/foo".into()))])),
					Err(Error::NotABranch("bar".into(), "please/let_me_in".into())),
				),
			},
			Test {
				description: "add node to a leaf node",
				input: (
					Node::Branch(Edges::new()),
					Vec::new(),
					"please/let_me_in".into(),
				),
				output: (Node::Branch(Edges::new()), Err(Error::EmptySegment)),
			},
			Test {
				description: "nodes don't get sorted anymore",
				input: (
					Node::Branch(Edges::from([
						("C".into(), Node::Leaf("1".into())),
						("Z".into(), Node::Leaf("2".into())),
						("B".into(), Node::Leaf("3".into())),
						("A".into(), Node::Leaf("4".into())),
					])),
					Vec::from([&capital_e[..]]),
					"5".into(),
				),
				output: (
					Node::Branch(Edges::from([
						("C".into(), Node::Leaf("1".into())),
						("Z".into(), Node::Leaf("2".into())),
						("B".into(), Node::Leaf("3".into())),
						("A".into(), Node::Leaf("4".into())),
						("E".into(), Node::Leaf("5".into())),
					])),
					Ok(()),
				),
			},
		]);

		for case in test_cases {
			let (mut tree, segments, link) = case.input;

			let result = tree.add(segments, link);
			let (want_tree, want_result) = case.output;

			assert_eq!(
				tree, want_tree,
				"mismatch when adding nodes: {:?}",
				case.description
			);
			assert_eq!(result, want_result);
		}
	}
}
