use std::{
	ffi::OsStr,
	path::{Path, PathBuf},
};

use indexmap::IndexMap;

use self::error::Error;

use self::iter::{Algorithm, Element, Iter, SearchAlgorithm};

pub mod error;
pub mod iter;

#[cfg(test)]
mod tests;

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

/// Alias for a node's edges.
pub type Edges = IndexMap<PathBuf, Node>;

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

		let (key, segments) = segments.unwrap();
		let key = PathBuf::from(key);

		match self {
			Self::Branch(self_children) => {
				let next = self_children.get_mut(&key);

				if segments.is_empty() {
					if next.is_some() {
						return Err(Error::LeafExists(key, link_path));
					}

					self_children.insert(key, Self::Leaf(link_path));
				} else if let Some(branch_node) = next {
					branch_node.add(segments.into(), link_path)?;
				} else {
					let mut branch_node = Self::Branch(IndexMap::new());
					branch_node.add(segments.into(), link_path)?;
					self_children.insert(key, branch_node);
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
	type IntoIter = Iter<'a, { Algorithm::DepthFirstSearch as SearchAlgorithm }>;

	fn into_iter(self) -> Self::IntoIter {
		self.into()
	}
}
