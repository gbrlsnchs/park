use std::{
	collections::{BTreeMap, HashSet},
	ffi::OsString,
	path::PathBuf,
};

pub type TargetMap = BTreeMap<PathBuf, Target>;

#[derive(Default)]
/// The main configuration for Park.
pub struct Config {
	pub defaults: Defaults,
	pub targets: TargetMap,
}

pub type TagSet = HashSet<String>;

#[derive(Default)]
/// These defaults should get applied to the command during runtime.
pub struct Defaults {
	pub tags: Option<TagSet>,
	pub base_dir: PathBuf,
}

#[derive(Default)]
/// Represents configuration for a dotfile.
pub struct Target {
	/// Link options of a dotfile.
	pub link: Option<Link>,
	/// Tags under which a dotfile should be managed.
	pub tags: Option<Tags>,
}

pub type TagList = Vec<String>;

#[derive(Default)]
/// Configuration for constraints that toggle certain dotfiles on and off.
pub struct Tags {
	/// These tags are evaluated conjunctively.
	pub all_of: Option<TagList>,
	/// These tags are evaluated disjunctively.
	pub any_of: Option<TagList>,
}

#[derive(Default)]
/// Configuration for the symlink of dotfiles.
pub struct Link {
	/// The place where the symlink gets created in.
	pub base_dir: Option<PathBuf>,
	/// Filename for the symlink.
	pub name: Option<OsString>,
}
