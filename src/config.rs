use std::{collections::HashMap, ffi::OsString, path::PathBuf};

/// The main configuration for Park.
pub struct Config {
	pub defaults: Defaults,
	pub targets: HashMap<PathBuf, Target>,
}

/// These defaults should get applied to the command during runtime.
pub struct Defaults {
	tags: Vec<String>,
	link: Link,
}

/// Represents configuration for a dotfile.
pub struct Target {
	/// Link options of a dotfile.
	pub link: Option<Link>,
	/// Tags under which a dotfile should be managed.
	pub tags: Option<Tags>,
}

#[derive(Default)]
/// Configuration for constraints that toggle certain dotfiles on and off.
pub struct Tags {
	/// These tags are evaluated conjunctively.
	pub all_of: Vec<String>,
	/// These tags are evaluated disjunctively.
	pub any_of: Vec<String>,
}

#[derive(Default)]
/// Configuration for the symlink of dotfiles.
pub struct Link {
	/// The place where the symlink gets created in.
	pub base_dir: PathBuf,
	/// Filename for the symlink.
	pub name: OsString,
}
