use std::{collections::HashMap, ffi::OsString, path::PathBuf};

/// The main configuration for Park.
pub struct Config {
	/// List of files meant to be linked.
	pub targets: Vec<PathBuf>,
	/// Optional list of options for targets.
	pub options: HashMap<PathBuf, Options>,
}

/// Represents all possible modifications that can be made to links.
#[derive(Default)]
pub struct Options {
	/// Optional alternative name for target's base directory.
	pub base_dir: Option<BaseDir>,
	/// Optional alternative name for target's link.
	pub link_name: Option<OsString>,
	/// Optional tags that conjunctively toggle a target on or off.
	pub conjunctive_tags: Option<Vec<String>>,
	/// Optional tags that disjunctively toggle a target on or off.
	pub disjunctive_tags: Option<Vec<String>>,
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
