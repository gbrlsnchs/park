use std::{ffi::OsString, path::PathBuf};

/// The main configuration for Park.
pub struct Config {
	/// List of files meant to be linked.
	pub targets: Vec<PathBuf>,
	/// List of options for targets.
	pub options: Options,
}

/// Represents all possible modifications that can be made to links.
#[derive(Default)]
pub struct Options {
	/// A different name for a target's link.
	pub link_name: OsString,
}
