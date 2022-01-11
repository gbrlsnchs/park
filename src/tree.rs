use std::{
	cell::RefCell,
	env,
	fmt::{Display, Error as FmtError, Formatter, Result as FmtResult},
	io::{Error as IoError, Write},
	path::PathBuf,
	rc::Rc,
	str,
};

use ansi_term::Colour;
use tabwriter::TabWriter;

use crate::{
	config::{Config, TagSet, Tags, Target},
	tree::node::Status,
};

use self::{iter::DepthFirstIter, node::NodeRef};
use self::{
	iter::NodeEntry,
	node::{AddError, Node},
};

mod iter;
mod node;

/// Structure representing all dotfiles after reading a configuration for Park.
#[derive(Debug, PartialEq)]
pub struct Tree {
	root: NodeRef,
	work_dir: PathBuf,
}

impl<'a> Tree {
	/// Parses a configuration and returns a tree based on it.
	pub fn parse(config: Config, mut runtime_tags: TagSet) -> Result<Self, AddError> {
		let targets = config.targets.unwrap_or_default();

		let cwd = env::current_dir().unwrap_or_default();
		let work_dir = config.work_dir.unwrap_or(cwd);

		let tree = Tree {
			root: Rc::new(RefCell::new(Node::Root(Vec::with_capacity(targets.len())))),
			work_dir,
		};

		let Config {
			base_dir: ref default_base_dir,
			tags: default_tags,
			..
		} = config;

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
	pub fn analyze(&self) -> Result<(), IoError> {
		for NodeEntry { node_ref, .. } in self {
			let mut node = node_ref.borrow_mut();

			match &mut *node {
				Node::Branch { path, .. } => {
					if path.exists() && path.is_file() {
						todo!();
					}
				}
				Node::Leaf {
					target_path,
					link_path,
					status,
				} => {
					let existing_target_path = link_path.read_link();

					if existing_target_path.is_err() {
						*status = if link_path.exists() {
							Status::Conflict
						} else {
							Status::Ready
						};

						continue;
					}

					let existing_target_path = existing_target_path.unwrap();

					let target_path = self.work_dir.join(target_path);

					*status = if existing_target_path == target_path {
						Status::Done
					} else {
						Status::Mismatch
					}
				}
				_ => {}
			}
		}

		Ok(())
	}
}

impl<'a> IntoIterator for &'a Tree {
	type Item = NodeEntry;
	type IntoIter = DepthFirstIter;

	fn into_iter(self) -> Self::IntoIter {
		DepthFirstIter::new(Rc::clone(&self.root))
	}
}

impl<'a> Display for Tree {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let table = Vec::new();
		let mut tab_writer = TabWriter::new(table).padding(1);

		let mut indent_blocks = Vec::<bool>::new();

		for NodeEntry {
			deepest,
			level,
			node_ref,
		} in self
		{
			let node = node_ref.borrow();

			if let Node::Root(..) = *node {
				let cwd = Colour::Cyan.paint(self.work_dir.to_string_lossy());
				if writeln!(tab_writer, ".\t== {}", cwd).is_err() {
					return Err(FmtError);
				}

				continue;
			}

			while level <= indent_blocks.len() {
				indent_blocks.pop();
			}

			indent_blocks.push(deepest);

			for (idx, has_indent_guide) in indent_blocks.iter().enumerate() {
				let is_leaf = idx == level - 1;

				let segment = match (has_indent_guide, is_leaf) {
					(true, true) => "└── ",
					(false, true) => "├── ",
					(true, _) => "    ",
					(false, _) => "│   ",
				};

				if write!(tab_writer, "{}", segment).is_err() {
					return Err(FmtError);
				}
			}

			match &*node {
				Node::Branch { path, .. } => {
					if writeln!(tab_writer, "{}\t\t", path.to_string_lossy()).is_err() {
						return Err(FmtError);
					};
				}
				Node::Leaf {
					target_path,
					link_path,
					status,
				} => {
					let status_str = format!("({:?})", status).to_uppercase();
					let status = match status {
						Status::Unknown => Colour::White.dimmed().paint(status_str),
						Status::Done => Colour::Blue.paint(status_str),
						Status::Ready => Colour::Green.paint(status_str),
						Status::Mismatch => Colour::Yellow.paint(status_str),
						Status::Conflict => Colour::Red.paint(status_str),
					};

					if writeln!(
						tab_writer,
						"{target_path}\t<- {link_path}\t{status}",
						target_path = target_path.file_name().unwrap().to_string_lossy(),
						link_path = Colour::Purple.paint(link_path.to_string_lossy()),
						status = status,
					)
					.is_err()
					{
						return Err(FmtError);
					};
				}
				_ => {}
			}
		}

		match tab_writer.into_inner() {
			Err(_) => return Err(FmtError),
			Ok(w) => {
				write!(f, "{}", str::from_utf8(&w).unwrap())?;
			}
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use indoc::indoc;
	use maplit::{btreemap, hashset};
	use pretty_assertions::assert_eq;

	use crate::{
		config::{Link, Tags},
		tree::node::Status,
	};

	use super::*;

	#[test]
	fn parse() -> Result<(), IoError> {
		struct Test<'a> {
			description: &'a str,
			input: (Config, TagSet),
			output: Result<Tree, AddError>,
		}

		let current_dir = env::current_dir()?;

		let test_cases = vec![
			Test {
				description: "simple config with a single target",
				input: (
					Config {
						targets: Some(btreemap! {
							PathBuf::from("foo") => Target::default()
						}),
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
					work_dir: PathBuf::from(&current_dir),
				}),
			},
			Test {
				description: "simple config with a nested target",
				input: (
					Config {
						targets: Some(btreemap! {
							PathBuf::from("foo/bar") => Target::default()
						}),
						..Config::default()
					},
					hashset! {},
				),
				output: Ok(Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Branch {
						path: PathBuf::from("foo"),
						children: vec![Node::new_ref(Node::Leaf {
							link_path: PathBuf::from("bar"),
							target_path: PathBuf::from("foo/bar"),
							status: Status::Unknown,
						})],
					})])),
					work_dir: PathBuf::from(&current_dir),
				}),
			},
			Test {
				description: "target with custom options",
				input: (
					Config {
						targets: Some(btreemap! {
							PathBuf::from("foo") => Target{
								link: Some(Link{
									name: Some(PathBuf::from("new_name")),
									..Link::default()
								}),
								..Target::default()
							}
						}),
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
					work_dir: PathBuf::from(&current_dir),
				}),
			},
			Test {
				description: "target disabled due to conjunctive tags",
				input: (
					Config {
						targets: Some(btreemap! {
							PathBuf::from("foo") => Target{
								tags: Some(Tags{
									all_of: Some(hashset!{String::from("test")}),
									any_of: Some(hashset!{String::from("foo"), String::from("bar")}),
								}),
								..Target::default()
							},
						}),
						..Config::default()
					},
					hashset! {
						String::from("foo"),
						String::from("bar"),
					},
				),
				output: Ok(Tree {
					root: Node::new_ref(Node::Root(vec![])),
					work_dir: PathBuf::from(&current_dir),
				}),
			},
			Test {
				description: "target enabled with tags #1",
				input: (
					Config {
						targets: Some(btreemap! {
							PathBuf::from("foo") => Target{
								tags: Some(Tags{
									all_of: Some(hashset!{String::from("test")}),
									..Tags::default()
								}),
								..Target::default()
							},
						}),
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
					work_dir: PathBuf::from(&current_dir),
				}),
			},
			Test {
				description: "target enabled with tags #2",
				input: (
					Config {
						targets: Some(btreemap! {
							PathBuf::from("foo") => Target{
								tags: Some(Tags{
									all_of: Some(hashset!{String::from("test")}),
									any_of: Some(hashset!{String::from("foo"), String::from("bar")}),
								}),
								..Target::default()
							},
						}),
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
					work_dir: PathBuf::from(&current_dir),
				}),
			},
			Test {
				description: "target disabled due to disjunctive tags",
				input: (
					Config {
						targets: Some(btreemap! {
							PathBuf::from("foo") => Target{
								tags: Some(Tags{
									all_of: Some(hashset!{String::from("test")}),
									any_of: Some(hashset!{String::from("foo"), String::from("bar")}),
								}),
								..Target::default()
							},
						}),
						..Config::default()
					},
					hashset! {
						String::from("test"),
					},
				),
				output: Ok(Tree {
					root: Node::new_ref(Node::Root(vec![])),
					work_dir: PathBuf::from(&current_dir),
				}),
			},
			Test {
				description: "target enabled with tags #3",
				input: (
					Config {
						targets: Some(btreemap! {
							PathBuf::from("foo") => Target{
								tags: Some(Tags{
									any_of: Some(hashset!{String::from("test")}),
									..Tags::default()
								}),
								..Target::default()
							},
						}),
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
					work_dir: PathBuf::from(&current_dir),
				}),
			},
		];

		for case in test_cases {
			let got = Tree::parse(case.input.0, case.input.1);

			assert_eq!(got, case.output, "bad result for {:?}", case.description);
		}

		Ok(())
	}

	#[test]
	fn analyze_tree() -> Result<(), IoError> {
		struct Test<'a> {
			description: &'a str,
			input: Tree,
			output: Tree,
		}

		let current_dir = env::current_dir()?;

		let test_cases = vec![
			Test {
				description: "single target should be ready",
				input: Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("foo"),
						target_path: PathBuf::from("foo"),
						status: Status::Unknown,
					})])),
					work_dir: PathBuf::from(&current_dir),
				},
				output: Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("foo"),
						target_path: PathBuf::from("foo"),
						status: Status::Ready,
					})])),
					work_dir: PathBuf::from(&current_dir),
				},
			},
			Test {
				description: "single target has conflict",
				input: Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("README.md"),
						target_path: PathBuf::from("Cargo.toml"),
						status: Status::Unknown,
					})])),
					work_dir: PathBuf::from(&current_dir),
				},
				output: Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("README.md"),
						target_path: PathBuf::from("Cargo.toml"),
						status: Status::Conflict,
					})])),
					work_dir: PathBuf::from(&current_dir),
				},
			},
			Test {
				description: "single target with wrong existing link",
				input: Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("tests/data/something"),
						target_path: PathBuf::from("something"),
						status: Status::Unknown,
					})])),
					work_dir: PathBuf::from(&current_dir),
				},
				output: Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("tests/data/something"),
						target_path: PathBuf::from("something"),
						status: Status::Mismatch,
					})])),
					work_dir: PathBuf::from(&current_dir),
				},
			},
			Test {
				description: "single target with correct existing link",
				input: Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("tests/data/something"),
						target_path: PathBuf::from("something"),
						status: Status::Unknown,
					})])),
					work_dir: PathBuf::from("test"),
				},
				output: Tree {
					root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("tests/data/something"),
						target_path: PathBuf::from("something"),
						status: Status::Done,
					})])),
					work_dir: PathBuf::from("test"),
				},
			},
		];

		for case in test_cases {
			case.input.analyze()?;

			assert_eq!(
				case.input, case.output,
				"bad result for {:?}",
				case.description
			);
		}

		Ok(())
	}

	#[test]
	fn format_tree() -> Result<(), IoError> {
		let tree = Tree {
			root: Node::new_ref(Node::Root(vec![
				Node::new_ref(Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("bar"),
						target_path: PathBuf::from("foo/bar"),
						status: Status::Unknown,
					})],
				}),
				Node::new_ref(Node::Branch {
					path: PathBuf::from("baz"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("test").join("qux"),
						target_path: PathBuf::from("baz/qux"),
						status: Status::Done,
					})],
				}),
				Node::new_ref(Node::Branch {
					path: PathBuf::from("quux"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("quuz"),
						target_path: PathBuf::from("quux/quuz"),
						status: Status::Ready,
					})],
				}),
				Node::new_ref(Node::Branch {
					path: PathBuf::from("corge"),
					children: vec![Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("test").join("gralt"),
						target_path: PathBuf::from("corge/gralt"),
						status: Status::Conflict,
					})],
				}),
			])),
			work_dir: PathBuf::from("test"),
		};

		println!("{}", tree);

		// TODO(gbrlsnchs): This can (and should) get better in the future. =)
		assert_eq!(
			tree.to_string(),
			format!(
				indoc! {"
					.             == {current_dir}
					├── foo                            
					│   └── bar   <- {bar}        {unknown}
					├── baz                            
					│   └── qux   <- {test_qux}   {done}
					├── quux                           
					│   └── quuz  <- {quuz}       {ready}
					└── corge                          
					    └── gralt <- {test_gralt} {conflict}
				"},
				current_dir = Colour::Cyan.paint("test"),
				bar = Colour::Purple.paint("bar"),
				test_qux = Colour::Purple.paint("test/qux"),
				quuz = Colour::Purple.paint("quuz"),
				test_gralt = Colour::Purple.paint("test/gralt"),
				unknown = Colour::White.dimmed().paint("(UNKNOWN)"),
				done = Colour::Blue.paint("(DONE)"),
				ready = Colour::Green.paint("(READY)"),
				conflict = Colour::Red.paint("(CONFLICT)"),
			)
		);

		Ok(())
	}
}
