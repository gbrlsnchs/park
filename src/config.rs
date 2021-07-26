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
	/// The base directory of a target's link.
	pub base_dir: BaseDir,
	/// A different name for a target's link.
	pub link_name: OsString,
}

/// Represents the base directory for a link. Its meaning is up to the application.
#[derive(Debug, PartialEq)]
pub enum BaseDir {
	Config,
	Cache,
	Data,
	Home,
	Bin,
	Documents,
	Download,
	Desktop,
	Pictures,
	Music,
	Videos,
	Templates,
	PublicShare,
}

impl Default for BaseDir {
	fn default() -> Self {
		Self::Config
	}
}
