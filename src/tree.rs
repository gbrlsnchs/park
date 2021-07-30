use crate::config::{Config, Options};

use self::node::{AddError, Node};

mod node;

/// Structure representing all dotfiles after reading a configuration for Park.
#[derive(Debug, PartialEq)]
struct Tree {
	pub(super) root: Node,
}

impl Tree {
	/// Parses a configuration and returns a tree based on it.
	fn parse(mut config: Config) -> Result<Self, AddError> {
		let mut tree = Tree {
			root: Node::Root(Vec::with_capacity(config.targets.len())),
		};
		for target in config.targets.into_iter() {
			// This "pops" the value out of the hash map, avoiding us to have to deal with a
			// borrowed value.
			let Options {
				base_dir,
				link_name,
			} = config.options.remove(&target).unwrap_or_default();

			tree.root.add(target, (base_dir, link_name))?;
		}

		Ok(tree)
	}
}

#[cfg(test)]
mod tests {
	use std::{collections::HashMap, ffi::OsString, path::PathBuf};

	use maplit::hashmap;
	use pretty_assertions::assert_eq;

	use crate::config::{BaseDir, Options};

	use super::*;

	#[test]
	fn simple_parsing() {
		struct Test<'a> {
			description: &'a str,
			input: Config,
			want: Result<Tree, AddError>,
		}

		let test_cases = vec![
			Test {
				description: "simple config with a single target",
				input: Config {
					targets: vec![PathBuf::from("foo")],
					options: HashMap::new(),
				},
				want: Ok(Tree {
					root: Node::Root(vec![Node::Leaf {
						base_dir: BaseDir::Config,
						link_name: OsString::from("foo"),
						path: PathBuf::from("foo"),
					}]),
				}),
			},
			Test {
				description: "simple config with a nested target",
				input: Config {
					targets: vec![PathBuf::from("foo/bar")],
					options: hashmap! {},
				},
				want: Ok(Tree {
					root: Node::Root(vec![Node::Branch {
						path: PathBuf::from("foo"),
						children: vec![Node::Leaf {
							base_dir: BaseDir::Config,
							link_name: OsString::from("bar"),
							path: PathBuf::from("bar"),
						}],
					}]),
				}),
			},
			Test {
				description: "target with custom options",
				input: Config {
					targets: vec![PathBuf::from("foo")],
					options: hashmap! {
						PathBuf::from("foo") => Options{
						    link_name: Some(OsString::from("new_name")),
							..Options::default()
						},
					},
				},
				want: Ok(Tree {
					root: Node::Root(vec![Node::Leaf {
						base_dir: BaseDir::Config,
						link_name: OsString::from("new_name"),
						path: PathBuf::from("foo"),
					}]),
				}),
			},
		];

		for case in test_cases.into_iter() {
			let got = Tree::parse(case.input);

			assert_eq!(got, case.want, "bad result for {:?}", case.description);
		}
	}
}
