use std::{
	collections::{BTreeMap, HashSet},
	path::PathBuf,
};

use serde::Deserialize;

pub type TargetMap = BTreeMap<PathBuf, Target>;
pub type TagSet = HashSet<String>;

#[derive(Debug, Default, Deserialize, PartialEq)]
/// The main configuration for Park.
pub struct Config {
	pub tags: Option<TagSet>,
	pub base_dir: PathBuf,
	pub work_dir: Option<PathBuf>,
	pub targets: Option<TargetMap>,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
/// Represents configuration for a dotfile.
pub struct Target {
	/// Link options of a dotfile.
	pub link: Option<Link>,
	/// Tags under which a dotfile should be managed.
	pub tags: Option<Tags>,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
/// Configuration for constraints that toggle certain dotfiles on and off.
pub struct Tags {
	/// These tags are evaluated conjunctively.
	pub all_of: Option<TagSet>,
	/// These tags are evaluated disjunctively.
	pub any_of: Option<TagSet>,
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
	use pretty_assertions::assert_eq;

	use super::*;

	#[test]
	fn deserialize_config_without_targets() {
		let got: Config = toml::from_str(indoc! {r#"
			tags = ["foo", "bar"]
			base_dir = "test"
		"#})
		.unwrap();

		assert_eq!(
			got,
			Config {
				tags: Some({
					let mut s = TagSet::new();
					s.insert(String::from("foo"));
					s.insert(String::from("bar"));
					s
				}),
				base_dir: PathBuf::from("test"),
				work_dir: None,
				targets: None,
			}
		);
	}

	#[test]
	fn deserialize_config_with_empty_targets() {
		let got: Config = toml::from_str(indoc! {r#"
			tags = ["foo", "bar"]
			base_dir = "test"
			work_dir = "somewhere"
			targets = {}
		"#})
		.unwrap();

		assert_eq!(
			got,
			Config {
				tags: Some({
					let mut s = TagSet::new();
					s.insert(String::from("foo"));
					s.insert(String::from("bar"));
					s
				}),
				base_dir: PathBuf::from("test"),
				work_dir: Some(PathBuf::from("somewhere")),
				targets: Some(TargetMap::new()),
			}
		);
	}

	#[test]
	fn deserialize_config_with_default_targets() {
		let got: Config = toml::from_str(indoc! {r#"
			tags = ["foo", "bar"]
			base_dir = "test"

			[targets.baz]

			[targets.qux]
		"#})
		.unwrap();

		assert_eq!(
			got,
			Config {
				tags: Some({
					let mut s = TagSet::new();
					s.insert(String::from("foo"));
					s.insert(String::from("bar"));
					s
				}),
				base_dir: PathBuf::from("test"),
				work_dir: None,
				targets: Some(TargetMap::from([
					(
						PathBuf::from("baz"),
						Target {
							link: None,
							tags: None,
						},
					),
					(
						PathBuf::from("qux"),
						Target {
							link: None,
							tags: None,
						},
					),
				])),
			}
		);
	}

	#[test]
	fn deserialize_config_with_custom_targets() {
		let got: Config = toml::from_str(indoc! {r#"
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
				tags: Some({
					let mut s = TagSet::new();
					s.insert(String::from("foo"));
					s.insert(String::from("bar"));
					s
				}),
				base_dir: PathBuf::from("test"),
				work_dir: None,
				targets: Some(TargetMap::from([
					(
						PathBuf::from("baz"),
						Target {
							link: Some(Link {
								name: Some(PathBuf::from("BAZ")),
								base_dir: None,
							}),
							tags: Some(Tags {
								all_of: Some(TagSet::from(["baz".into()])),
								any_of: None,
							}),
						},
					),
					(
						PathBuf::from("qux"),
						Target {
							link: Some(Link {
								name: None,
								base_dir: Some(PathBuf::from("elsewhere")),
							}),
							tags: Some(Tags {
								all_of: None,
								any_of: Some(TagSet::from(["qux".into()])),
							}),
						},
					),
				])),
			}
		);
	}
}
