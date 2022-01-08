use std::{
	collections::{BTreeMap, HashSet},
	path::PathBuf,
};

use serde::Deserialize;

pub type TargetMap = BTreeMap<PathBuf, Target>;

#[derive(Debug, Default, Deserialize, PartialEq)]
/// The main configuration for Park.
pub struct Config {
	pub tags: Option<TagSet>,
	pub base_dir: PathBuf,
	pub targets: Option<TargetMap>,
}

pub type TagSet = HashSet<String>;

#[derive(Debug, Default, Deserialize, PartialEq)]
/// Represents configuration for a dotfile.
pub struct Target {
	/// Link options of a dotfile.
	pub link: Option<Link>,
	/// Tags under which a dotfile should be managed.
	pub tags: Option<Tags>,
}

pub type TagList = HashSet<String>;

#[derive(Debug, Default, Deserialize, PartialEq)]
/// Configuration for constraints that toggle certain dotfiles on and off.
pub struct Tags {
	/// These tags are evaluated conjunctively.
	pub all_of: Option<TagList>,
	/// These tags are evaluated disjunctively.
	pub any_of: Option<TagList>,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
/// Configuration for the symlink of dotfiles.
pub struct Link {
	/// The place where the symlink gets created in.
	pub base_dir: Option<PathBuf>,
	/// Filename for the symlink.
	pub name: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
	use indoc::indoc;
	use maplit::{btreemap, hashset};
	use pretty_assertions::assert_eq;
	use toml::from_str;

	use super::*;

	#[test]
	fn deserialize_config_without_targets() {
		let got: Config = from_str(indoc! {r#"
			tags = ["foo", "bar"]
			base_dir = "test"
		"#})
		.unwrap();

		assert_eq!(
			got,
			Config {
				tags: Some(hashset! {String::from("foo"), String::from("bar")}),
				base_dir: PathBuf::from("test"),
				targets: None,
			}
		);
	}

	#[test]
	fn deserialize_config_with_empty_targets() {
		let got: Config = from_str(indoc! {r#"
			tags = ["foo", "bar"]
			base_dir = "test"
			targets = {}
		"#})
		.unwrap();

		assert_eq!(
			got,
			Config {
				tags: Some(hashset! {String::from("foo"), String::from("bar")}),
				base_dir: PathBuf::from("test"),
				targets: Some(btreemap! {}),
			}
		);
	}

	#[test]
	fn deserialize_config_with_default_targets() {
		let got: Config = from_str(indoc! {r#"
			tags = ["foo", "bar"]
			base_dir = "test"

			[targets.baz]

			[targets.qux]
		"#})
		.unwrap();

		assert_eq!(
			got,
			Config {
				tags: Some(hashset! {String::from("foo"), String::from("bar")}),
				base_dir: PathBuf::from("test"),
				targets: Some(btreemap! {
					PathBuf::from("baz") => Target{
						link: None,
						tags: None,
					},
					PathBuf::from("qux") => Target{
						link: None,
						tags: None,
					},
				}),
			}
		);
	}

	#[test]
	fn deserialize_config_with_custom_targets() {
		let got: Config = from_str(indoc! {r#"
			tags = ["foo", "bar"]
			base_dir = "test"

			[targets.baz]
			link.name = "BAZ"
			tags.all_of = ["baz"]

			[targets.qux]
			link.base_dir = "elsewhere"
			tags.any_of = ["qux"]
		"#})
		.unwrap();

		assert_eq!(
			got,
			Config {
				tags: Some(hashset! {String::from("foo"), String::from("bar")}),
				base_dir: PathBuf::from("test"),
				targets: Some(btreemap! {
					PathBuf::from("baz") => Target{
						link: Some(Link{
							name: Some(PathBuf::from("BAZ")),
							base_dir: None,
						}),
						tags: Some(Tags{
							all_of: Some(hashset!{String::from("baz")}),
							any_of: None,
						}),
					},
					PathBuf::from("qux") => Target{
						link: Some(Link{
							name: None,
							base_dir: Some(PathBuf::from("elsewhere")),
						}),
						tags: Some(Tags{
							all_of: None,
							any_of: Some(hashset!{String::from("qux")}),
						}),
					},
				}),
			}
		);
	}
}
