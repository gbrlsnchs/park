use std::{
	cell::RefCell,
	fmt::{Display, Formatter, Result as FmtResult},
	rc::Rc,
};

use crate::{
	config::{Config, Defaults, TagSet, Tags, Target},
	tree::node::Status,
};

use self::node::{AddError, Node};
use self::{iter::DepthFirstIter, node::NodeRef};

mod iter;
mod node;

/// Structure representing all dotfiles after reading a configuration for Park.
#[derive(Debug, PartialEq)]
struct Tree {
	pub(super) root: NodeRef,
}

impl<'a> Tree {
	/// Parses a configuration and returns a tree based on it.
	pub fn parse(config: Config, mut runtime_tags: TagSet) -> Result<Self, AddError> {
		let targets = config.targets;

		let tree = Tree {
			root: Rc::new(RefCell::new(Node::Root(Vec::with_capacity(targets.len())))),
		};

		let Defaults {
			base_dir: ref default_base_dir,
			tags: default_tags,
		} = config.defaults;

		if let Some(default_tags) = default_tags {
			runtime_tags.extend(default_tags);
		}

		'targets: for (target_path, target) in targets {
			let Target {
				link,
				tags: target_tags,
			} = target;

			let target_tags = target_tags.unwrap_or_default();

			let Tags { all_of, any_of } = target_tags;
			let (all_of, any_of) = (all_of.unwrap_or_default(), any_of.unwrap_or_default());

			let mut allowed = true;
			for tag in &all_of {
				allowed = allowed && runtime_tags.contains(tag);

				if !allowed {
					continue 'targets;
				}
			}

			// No disjunctive tags? Pass.
			let mut allowed = any_of.is_empty();
			for tag in &any_of {
				allowed = allowed || runtime_tags.contains(tag);
			}
			if !allowed {
				continue;
			}

			tree.root
				.borrow_mut()
				.add(default_base_dir, target_path, link.unwrap_or_default())?;
		}

		Ok(tree)
	}

	/// Analyze the tree's nodes in order to check viability for symlinks to be done.
	/// This means it will iterate the tree and update each node's status.
	pub fn analyze(&self) {
		for node_ref in self {
			let mut node = node_ref.borrow_mut();

			match &mut *node {
				Node::Branch { path, .. } => {
					if path.exists() && path.is_file() {
						todo!();
					}
				}
				Node::Leaf {
					target_path,
					status,
					..
				} => {
					if target_path.exists() {
						*status = Status::Conflict;
					} else {
						*status = Status::Ready;
					}
				}
				_ => {}
			}
		}
	}
}

impl<'a> IntoIterator for &'a Tree {
	type Item = NodeRef;
	type IntoIter = DepthFirstIter;

	fn into_iter(self) -> Self::IntoIter {
		DepthFirstIter::new(Rc::clone(&self.root))
	}
}

impl<'a> Display for Tree {
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
						write(f, &child.borrow(), indent_boundaries)?;
						indent_boundaries.pop();
					}
				}
				Node::Branch { path, children } => {
					indent(f, indent_boundaries)?;
					writeln!(f, "{}", path.to_string_lossy())?;

					for (idx, child) in children.iter().enumerate() {
						indent_boundaries.push(idx == children.len() - 1);
						write(f, &child.borrow(), indent_boundaries)?;
						indent_boundaries.pop();
					}
				}
				Node::Leaf {
					target_path,
					link_path,
					..
				} => {
					indent(f, indent_boundaries)?;

					writeln!(
						f,
						"{target_path} <- {link_path}",
						target_path = target_path.to_string_lossy(),
						link_path = link_path.to_string_lossy(),
					)?;
				}
			}

			Ok(())
		}

		write(f, &self.root.borrow(), &mut indent_boundaries)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::{ffi::OsString, path::PathBuf};

	use maplit::{btreemap, hashset};
	use pretty_assertions::assert_eq;

	use crate::{
		config::{Link, Tags},
		tree::node::Status,
	};

	use super::*;

	#[test]
	fn parse() {
		struct Test<'a> {
			description: &'a str,
			input: (Config, TagSet),
			output: Result<Tree, AddError>,
		}

		let test_cases = vec![
			Test {
				description: "simple config with a single target",
				input: (
					Config {
						targets: btreemap! {
							PathBuf::from("foo") => Target::default()
						},
						..Config::default()
					},
					hashset! {},
				),
				output: Ok(Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("foo"),
						target_path: PathBuf::from("foo"),
						status: Status::Unknown,
					})])),
				}),
			},
			Test {
				description: "simple config with a nested target",
				input: (
					Config {
						targets: btreemap! {
							PathBuf::from("foo/bar") => Target::default()
						},
						..Config::default()
					},
					hashset! {},
				),
				output: Ok(Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Branch {
						path: PathBuf::from("foo"),
						children: vec![Node::new_ref(Node::Leaf {
							link_path: PathBuf::from("bar"),
							target_path: PathBuf::from("bar"),
							status: Status::Unknown,
						})],
					})])),
				}),
			},
			Test {
				description: "target with custom options",
				input: (
					Config {
						targets: btreemap! {
							PathBuf::from("foo") => Target{
								link: Some(Link{
									name: Some(OsString::from("new_name")),
									..Link::default()
								}),
								..Target::default()
							}
						},
						..Config::default()
					},
					hashset! {},
				),
				output: Ok(Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("new_name"),
						target_path: PathBuf::from("foo"),
						status: Status::Unknown,
					})])),
				}),
			},
			Test {
				description: "target disabled due to conjunctive tags",
				input: (
					Config {
						targets: btreemap! {
							PathBuf::from("foo") => Target{
								tags: Some(Tags{
									all_of: Some(vec![String::from("test")]),
									any_of: Some(vec![String::from("foo"), String::from("bar")]),
								}),
								..Target::default()
							},
						},
						..Config::default()
					},
					hashset! {
						String::from("foo"),
						String::from("bar"),
					},
				),
				output: Ok(Tree {
					root: Node::new_ref(Node::Root(vec![])),
				}),
			},
			Test {
				description: "target enabled with tags #1",
				input: (
					Config {
						targets: btreemap! {
							PathBuf::from("foo") => Target{
								tags: Some(Tags{
									all_of: Some(vec![String::from("test")]),
									..Tags::default()
								}),
								..Target::default()
							},
						},
						..Config::default()
					},
					hashset! {
						String::from("test"),
					},
				),
				output: Ok(Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("foo"),
						target_path: PathBuf::from("foo"),
						status: Status::Unknown,
					})])),
				}),
			},
			Test {
				description: "target enabled with tags #2",
				input: (
					Config {
						targets: btreemap! {
							PathBuf::from("foo") => Target{
								tags: Some(Tags{
									all_of: Some(vec![String::from("test")]),
									any_of: Some(vec![String::from("foo"), String::from("bar")]),
								}),
								..Target::default()
							},
						},
						..Config::default()
					},
					hashset! {
						String::from("test"),
						String::from("bar"),
					},
				),
				output: Ok(Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("foo"),
						target_path: PathBuf::from("foo"),
						status: Status::Unknown,
					})])),
				}),
			},
			Test {
				description: "target disabled due to disjunctive tags",
				input: (
					Config {
						targets: btreemap! {
							PathBuf::from("foo") => Target{
								tags: Some(Tags{
									all_of: Some(vec![String::from("test")]),
									any_of: Some(vec![String::from("foo"), String::from("bar")]),
								}),
								..Target::default()
							},
						},
						..Config::default()
					},
					hashset! {
						String::from("test"),
					},
				),
				output: Ok(Tree {
					root: Node::new_ref(Node::Root(vec![])),
				}),
			},
			Test {
				description: "target enabled with tags #3",
				input: (
					Config {
						targets: btreemap! {
							PathBuf::from("foo") => Target{
								tags: Some(Tags{
									any_of: Some(vec![String::from("test")]),
									..Tags::default()
								}),
								..Target::default()
							},
						},
						..Config::default()
					},
					hashset! {
						String::from("test"),
					},
				),
				output: Ok(Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("foo"),
						target_path: PathBuf::from("foo"),
						status: Status::Unknown,
					})])),
				}),
			},
		];

		for case in test_cases {
			let got = Tree::parse(case.input.0, case.input.1);

			assert_eq!(got, case.output, "bad result for {:?}", case.description);
		}
	}

	#[test]
	fn analyze() {
		struct Test<'a> {
			description: &'a str,
			input: Tree,
			output: Tree,
		}

		let test_cases = vec![Test {
			description: "single target should be ready",
			input: Tree {
				root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
					link_path: PathBuf::from("foo"),
					target_path: PathBuf::from("foo"),
					status: Status::Unknown,
				})])),
			},
			output: Tree {
				root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
					link_path: PathBuf::from("foo"),
					target_path: PathBuf::from("foo"),
					status: Status::Ready,
				})])),
			},
		}];

		for case in test_cases {
			case.input.analyze();

			assert_eq!(
				case.input, case.output,
				"bad result for {:?}",
				case.description
			);
		}
	}

	#[test]
	fn format_tree() {
		let tree = Tree {
			root: Node::new_ref(Node::Root(vec![
				Node::new_ref(Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("bar"),
						target_path: PathBuf::from("bar"),
						status: Status::Unknown,
					})],
				}),
				Node::new_ref(Node::Branch {
					path: PathBuf::from("qux"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("test").join("quux"),
						target_path: PathBuf::from("quux"),
						status: Status::Unknown,
					})],
				}),
			])),
		};

		println!("{}", tree);

		// TODO(gbrlsnchs): This can (and should) get better in the future. =)
		assert_eq!(
			tree.to_string(),
			concat!(
				".\n",
				"├── foo\n",
				"│   └── bar <- bar\n",
				"└── qux\n",
				"    └── quux <- test/quux\n",
			)
		);
	}
}
