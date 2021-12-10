use std::{
	collections::HashSet,
	fmt::{Display, Formatter, Result as FmtResult},
	vec::IntoIter,
};

use crate::config::{Config, Target};

use self::node::{AddError, Node};

mod node;

/// String values used to toggle nodes on and off.
pub type Tags = HashSet<String>;

/// Structure representing all dotfiles after reading a configuration for Park.
#[derive(Debug, PartialEq)]
struct Tree {
	pub(super) root: Node,
}

impl Tree {
	/// Parses a configuration and returns a tree based on it.
	pub fn parse(config: Config, tags: Tags) -> Result<Self, AddError> {
		let mut tree = Tree {
			root: Node::Root(Vec::with_capacity(config.targets.len())),
		};
		for (target_path, target) in config.targets {
			let Target {
				link,
				tags: target_tags,
			} = target;

			let node_tags = target_tags.unwrap_or_default();
			let mut allowed = true;

			for tag in &node_tags.all_of {
				allowed = allowed && tags.contains(tag);
			}

			if !allowed {
				continue;
			}

			let mut allowed = node_tags.all_of.is_empty();

			for tag in &node_tags.any_of {
				allowed = allowed || tags.contains(tag);
			}

			if !allowed {
				continue;
			}

			tree.root.add(target_path, link.unwrap_or_default())?;
		}

		Ok(tree)
	}
}

impl<'a> IntoIterator for &'a Tree {
	type Item = &'a Node;
	type IntoIter = IntoIter<&'a Node>;

	/// Returns a depth first iterator.
	fn into_iter(self) -> Self::IntoIter {
		/// Recursively builds a stack of nodes using depth first search.
		fn append<'a>(stack: &mut Vec<&'a Node>, node: &'a Node) {
			stack.push(node);

			match node {
				Node::Root(children) | Node::Branch { children, .. } => {
					for child in children {
						append(stack, child);
					}
				}
				_ => {}
			}
		}

		let mut stack = vec![];
		append(&mut stack, &self.root);

		stack.into_iter()
	}
}

impl Display for Tree {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let mut indent_boundaries = Vec::new();

		/// This will build the line for the node, filling it with correct symbols, like in
		///
		/// \ \ .
		/// \ \ ├── A
		/// \ \ │   └── B
		/// \ \ ├── C
		/// \ \ └── D
		/// \ \ \    └── E
		///
		///  where "│   └── B" is considered a line, for example.
		fn indent<'a>(f: &mut Formatter<'_>, indent_boundaries: &'a mut Vec<bool>) -> FmtResult {
			let mut prefix;
			for (idx, is_last) in indent_boundaries.iter().enumerate() {
				let is_boundary = *is_last;
				let is_rightmost = idx == indent_boundaries.len() - 1;

				if !is_rightmost {
					prefix = "│";

					if is_boundary {
						prefix = " ";
					}

					write!(f, "{}   ", prefix)?;

					continue;
				}

				prefix = "├";

				if is_boundary {
					prefix = "└";
				}

				write!(f, "{}── ", prefix)?;
			}

			Ok(())
		}

		fn write<'a>(
			f: &mut Formatter<'_>,
			node: &'a Node,
			indent_boundaries: &'a mut Vec<bool>,
		) -> FmtResult {
			match node {
				Node::Root(children) => {
					writeln!(f, ".")?;

					for (idx, child) in children.iter().enumerate() {
						indent_boundaries.push(idx == children.len() - 1);
						write(f, child, indent_boundaries)?;
						indent_boundaries.pop();
					}
				}
				Node::Branch { path, children } => {
					indent(f, indent_boundaries)?;
					writeln!(f, "{}", path.to_string_lossy())?;

					for (idx, child) in children.iter().enumerate() {
						indent_boundaries.push(idx == children.len() - 1);
						write(f, child, indent_boundaries)?;
						indent_boundaries.pop();
					}
				}
				Node::Leaf {
					path,
					base_dir,
					link_name,
				} => {
					indent(f, indent_boundaries)?;
					writeln!(
						f,
						"{path} <- {base_dir:?}/{link_name}",
						path = path.to_string_lossy(),
						base_dir = base_dir,
						link_name = link_name.to_string_lossy()
					)?;
				}
			}

			Ok(())
		}

		write(f, &self.root, &mut indent_boundaries)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::{ffi::OsString, path::PathBuf};

	use maplit::{hashmap, hashset};
	use pretty_assertions::assert_eq;

	use crate::config::{BaseDir, Options};

	use super::*;

	#[test]
	fn simple_parsing() {
		struct Test<'a> {
			description: &'a str,
			input: (Config, Tags),
			want: Result<Tree, AddError>,
		}

		let test_cases = vec![
			Test {
				description: "simple config with a single target",
				input: (
					Config {
						targets: vec![PathBuf::from("foo")],
						options: hashmap! {},
					},
					hashset! {},
				),
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
				input: (
					Config {
						targets: vec![PathBuf::from("foo/bar")],
						options: hashmap! {},
					},
					hashset! {},
				),
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
				input: (
					Config {
						targets: vec![PathBuf::from("foo")],
						options: hashmap! {
							PathBuf::from("foo") => Options{
								link_name: Some(OsString::from("new_name")),
								..Options::default()
							},
						},
					},
					hashset! {},
				),
				want: Ok(Tree {
					root: Node::Root(vec![Node::Leaf {
						base_dir: BaseDir::Config,
						link_name: OsString::from("new_name"),
						path: PathBuf::from("foo"),
					}]),
				}),
			},
			Test {
				description: "target disabled due to conjunctive tags",
				input: (
					Config {
						targets: vec![PathBuf::from("foo")],
						options: hashmap! {
							PathBuf::from("foo") => Options{
								conjunctive_tags: Some(vec![String::from("test")]),
								disjunctive_tags: Some(vec![
									String::from("foo"),
									String::from("bar"),
								]),
								..Options::default()
							},
						},
					},
					hashset! {
						String::from("foo"),
						String::from("bar"),
					},
				),
				want: Ok(Tree {
					root: Node::Root(vec![]),
				}),
			},
			Test {
				description: "target enabled with tags #1",
				input: (
					Config {
						targets: vec![PathBuf::from("foo")],
						options: hashmap! {
							PathBuf::from("foo") => Options{
								conjunctive_tags: Some(vec![String::from("test")]),
								..Options::default()
							},
						},
					},
					hashset! {
						String::from("test"),
					},
				),
				want: Ok(Tree {
					root: Node::Root(vec![Node::Leaf {
						base_dir: BaseDir::Config,
						link_name: OsString::from("foo"),
						path: PathBuf::from("foo"),
					}]),
				}),
			},
			Test {
				description: "target enabled with tags #2",
				input: (
					Config {
						targets: vec![PathBuf::from("foo")],
						options: hashmap! {
							PathBuf::from("foo") => Options{
								conjunctive_tags: Some(vec![String::from("test")]),
								disjunctive_tags: Some(vec![
									String::from("foo"),
									String::from("bar"),
								]),
								..Options::default()
							},
						},
					},
					hashset! {
						String::from("test"),
						String::from("bar"),
					},
				),
				want: Ok(Tree {
					root: Node::Root(vec![Node::Leaf {
						base_dir: BaseDir::Config,
						link_name: OsString::from("foo"),
						path: PathBuf::from("foo"),
					}]),
				}),
			},
			Test {
				description: "target disabled due to disjunctive tags",
				input: (
					Config {
						targets: vec![PathBuf::from("foo")],
						options: hashmap! {
							PathBuf::from("foo") => Options{
								conjunctive_tags: Some(vec![String::from("test")]),
								disjunctive_tags: Some(vec![
									String::from("foo"),
									String::from("bar"),
								]),
								..Options::default()
							},
						},
					},
					hashset! {
						String::from("test"),
					},
				),
				want: Ok(Tree {
					root: Node::Root(vec![]),
				}),
			},
			Test {
				description: "target enabled with tags #3",
				input: (
					Config {
						targets: vec![PathBuf::from("foo")],
						options: hashmap! {
							PathBuf::from("foo") => Options{
								disjunctive_tags: Some(vec![String::from("test")]),
								..Options::default()
							},
						},
					},
					hashset! {
						String::from("test"),
					},
				),
				want: Ok(Tree {
					root: Node::Root(vec![Node::Leaf {
						base_dir: BaseDir::Config,
						link_name: OsString::from("foo"),
						path: PathBuf::from("foo"),
					}]),
				}),
			},
		];

		for case in test_cases {
			let got = Tree::parse(case.input.0, case.input.1);

			assert_eq!(got, case.want, "bad result for {:?}", case.description);
		}
	}

	#[test]
	fn depth_first_iterator() {
		let tree = Tree {
			root: Node::Root(vec![
				Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::Leaf {
						base_dir: BaseDir::Config,
						link_name: OsString::from("bar"),
						path: PathBuf::from("bar"),
					}],
				},
				Node::Branch {
					path: PathBuf::from("qux"),
					children: vec![Node::Leaf {
						base_dir: BaseDir::Config,
						link_name: OsString::from("quux"),
						path: PathBuf::from("quux"),
					}],
				},
			]),
		};

		let got = tree.into_iter().collect::<Vec<&Node>>();

		assert_eq!(
			got,
			vec![
				&Node::Root(vec![
					Node::Branch {
						path: PathBuf::from("foo"),
						children: vec![Node::Leaf {
							base_dir: BaseDir::Config,
							link_name: OsString::from("bar"),
							path: PathBuf::from("bar"),
						}],
					},
					Node::Branch {
						path: PathBuf::from("qux"),
						children: vec![Node::Leaf {
							base_dir: BaseDir::Config,
							link_name: OsString::from("quux"),
							path: PathBuf::from("quux"),
						}],
					},
				]),
				&Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::Leaf {
						base_dir: BaseDir::Config,
						link_name: OsString::from("bar"),
						path: PathBuf::from("bar"),
					}],
				},
				&Node::Leaf {
					base_dir: BaseDir::Config,
					link_name: OsString::from("bar"),
					path: PathBuf::from("bar"),
				},
				&Node::Branch {
					path: PathBuf::from("qux"),
					children: vec![Node::Leaf {
						base_dir: BaseDir::Config,
						link_name: OsString::from("quux"),
						path: PathBuf::from("quux"),
					}],
				},
				&Node::Leaf {
					base_dir: BaseDir::Config,
					link_name: OsString::from("quux"),
					path: PathBuf::from("quux"),
				},
			]
		);
	}

	#[test]
	fn format_tree() {
		let tree = Tree {
			root: Node::Root(vec![
				Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::Leaf {
						base_dir: BaseDir::Config,
						link_name: OsString::from("bar"),
						path: PathBuf::from("bar"),
					}],
				},
				Node::Branch {
					path: PathBuf::from("qux"),
					children: vec![Node::Leaf {
						base_dir: BaseDir::Config,
						link_name: OsString::from("quux"),
						path: PathBuf::from("quux"),
					}],
				},
			]),
		};

		println!("{}", tree);

		// TODO(gbrlsnchs): This can (and should) get better in the future. =)
		assert_eq!(
			tree.to_string(),
			concat!(
				".\n",
				"├── foo\n",
				"│   └── bar <- Config/bar\n",
				"└── qux\n",
				"    └── quux <- Config/quux\n",
			)
		);
	}
}
