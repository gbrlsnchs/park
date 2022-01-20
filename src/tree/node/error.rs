use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum Error {
	#[error("node for link {1:?} at segment {0:?} cannot be inserted because it is not a branch")]
	NotABranch(PathBuf, PathBuf),
	#[error("node for link {1:?} at segment {0:?} already exists as a leaf")]
	LeafExists(PathBuf, PathBuf),
	#[error("cannot add empty link path")]
	EmptySegment,
}
