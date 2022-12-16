use std::{
	collections::{HashMap, HashSet},
	env, fs,
	io::Error as IoError,
	os::unix::fs as unix_fs,
	path::PathBuf,
};

use crate::config::{Config, TagSet, Tags, Target};

use super::{
	error::Error,
	iter::Element as IterElement,
	node::{Edges, Error as NodeError, Node, Status},
};

pub type Statuses = HashMap<PathBuf, Status>;

/// Structure representing all dotfiles after reading a configuration for Park.
#[derive(Debug, PartialEq)]
pub struct Tree {
	pub root: Node,
	pub work_dir: PathBuf,
	pub statuses: Statuses,
}

impl<'a> Tree {
	/// Parses a configuration and returns a tree based on it.
	pub fn parse(config: Config, filters: (TagSet, HashSet<PathBuf>)) -> Result<Self, NodeError> {
		let (mut runtime_tags, target_filters) = filters;
		let targets = config.targets.unwrap_or_default();

		let cwd = env::current_dir().unwrap_or_default();
		let work_dir = config.work_dir.unwrap_or(cwd);

		let mut tree = Tree {
			root: Node::Branch(Edges::new()),
			work_dir,
			statuses: Statuses::new(),
		};

		let Config {
			base_dir: default_base_dir,
			tags: default_tags,
			..
		} = config;

		let default_base_dir = default_base_dir.unwrap_or_default();

		if let Some(default_tags) = default_tags {
			runtime_tags.extend(default_tags);
		}

		for (target_path, target) in targets {
			if !target_filters.is_empty() && !target_filters.contains(&target_path) {
				continue;
			}

			let Target {
				link,
				tags: target_tags,
			} = target;

			let target_tags = target_tags.unwrap_or_default();

			let Tags { all_of, any_of } = target_tags;
			let (all_of, any_of) = (all_of.unwrap_or_default(), any_of.unwrap_or_default());

			if !all_of.is_empty() && !all_of.iter().all(|tag| runtime_tags.contains(tag)) {
				continue;
			}

			if !any_of.is_empty() && !any_of.iter().any(|tag| runtime_tags.contains(tag)) {
				continue;
			}

			let link = link.unwrap_or_default();
			let base_dir = link.base_dir.as_ref().unwrap_or(&default_base_dir);
			let link_path = link.name.map_or_else(
				|| {
					target_path
						.file_name()
						.map(|file_name| base_dir.join(file_name))
						.unwrap()
				},
				|name| base_dir.join(name),
			);
			tree.root.add(target_path.iter().collect(), link_path)?;
		}

		Ok(tree)
	}

	/// Analyze the tree's nodes in order to check viability for symlinks to be done.
	/// This means it will iterate the tree and update each node's status.
	pub fn analyze(&mut self) -> Result<(), IoError> {
		let Tree {
			ref mut statuses,
			ref root,
			..
		} = self;

		for IterElement {
			link_path,
			target_path,
			..
		} in root
		{
			if let Some(link_path) = link_path {
				if let Some(parent_dir) = link_path.parent() {
					if parent_dir.exists() && !parent_dir.is_dir() {
						statuses.insert(link_path, Status::Obstructed);

						continue;
					}
				}

				let existing_target_path = link_path.read_link();

				if existing_target_path.is_err() {
					let link_exists = link_path.exists();
					let link_parent_exists = link_path.parent().map_or(true, |parent| {
						parent.as_os_str().is_empty() || parent.exists()
					});
					statuses.insert(
						link_path,
						if link_exists {
							Status::Conflict
						} else if link_parent_exists {
							Status::Ready
						} else {
							Status::Unparented
						},
					);

					continue;
				}

				let existing_target_path = existing_target_path.unwrap();

				let target_path = self.work_dir.join(target_path);

				statuses.insert(
					link_path,
					if existing_target_path == target_path {
						Status::Done
					} else {
						Status::Mismatch
					},
				);
			}
		}

		Ok(())
	}

	pub fn link(&self) -> Result<(), Error> {
		let links: Result<Vec<(PathBuf, PathBuf)>, Error> = self
			.root
			.into_iter()
			.filter(|IterElement { link_path, .. }| link_path.is_some()) // filters branches
			.filter(|IterElement { link_path, .. }| {
				match self.statuses.get(link_path.as_ref().unwrap()) {
					Some(Status::Done) => false,
					_ => true,
				}
			})
			.map(
				|IterElement {
				     target_path,
				     link_path,
				     ..
				 }| {
					let link_path = link_path.unwrap();

					if let Some(status) = self.statuses.get(&link_path) {
						match status {
							// TODO: Return more detailed errors.
							Status::Unknown
							| Status::Mismatch
							| Status::Conflict
							| Status::Obstructed => return Err(Error::InternalError(link_path)),
							Status::Unparented => {
								if let Some(link_parent_dir) = link_path.parent() {
									if let Err(err) = fs::create_dir_all(link_parent_dir) {
										return Err(Error::IoError(err.kind()));
									}
								}
							}
							_ => {}
						}

						return Ok((self.work_dir.join(target_path), link_path.clone()));
					}

					unreachable!();
				},
			)
			.collect();

		let mut created_links = Vec::new();
		for (target_path, link_path) in links? {
			if let Err(err) = unix_fs::symlink(target_path, &link_path) {
				return Err(Error::IoError(err.kind()));
			};

			created_links.push(link_path);
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::{fs, path::PathBuf};

	use pretty_assertions::assert_eq;

	use crate::config::{Link, TagSet, Tags, TargetMap};

	use super::*;

	#[test]
	fn parse() -> Result<(), IoError> {
		struct Test<'a> {
			description: &'a str,
			input: (Config, (TagSet, HashSet<PathBuf>)),
			output: Result<Tree, NodeError>,
		}

		let current_dir = &env::current_dir()?;

		let test_cases = Vec::from([
			Test {
				description: "simple config with a single target",
				input: (
					Config {
						targets: Some(TargetMap::from([("foo".into(), Target::default())])),
						..Config::default()
					},
					(TagSet::from([]), HashSet::from([])),
				),
				output: Ok(Tree {
					root: Node::Branch(Edges::from([("foo".into(), Node::Leaf("foo".into()))])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				}),
			},
			Test {
				description: "simple config with a nested target",
				input: (
					Config {
						targets: Some(TargetMap::from([("foo/bar".into(), Target::default())])),
						..Config::default()
					},
					(TagSet::from([]), HashSet::from([])),
				),
				output: Ok(Tree {
					root: Node::Branch(Edges::from([(
						"foo".into(),
						Node::Branch(Edges::from([("bar".into(), Node::Leaf("bar".into()))])),
					)])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				}),
			},
			Test {
				description: "target with custom options",
				input: (
					Config {
						targets: Some(TargetMap::from([(
							"foo".into(),
							Target {
								link: Some(Link {
									name: Some("new_name".into()),
									..Link::default()
								}),
								..Target::default()
							},
						)])),
						..Config::default()
					},
					(TagSet::from([]), HashSet::from([])),
				),
				output: Ok(Tree {
					root: Node::Branch(Edges::from([(
						"foo".into(),
						Node::Leaf("new_name".into()),
					)])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				}),
			},
			Test {
				description: "target disabled due to conjunctive tags",
				input: (
					Config {
						targets: Some(TargetMap::from([(
							"foo".into(),
							Target {
								tags: Some(Tags {
									all_of: Some(TagSet::from(["test".into()])),
									any_of: Some(TagSet::from(["foo/bar".into()])),
								}),
								..Target::default()
							},
						)])),
						..Config::default()
					},
					(
						TagSet::from(["foo".into(), "bar".into()]),
						HashSet::from([]),
					),
				),
				output: Ok(Tree {
					root: Node::Branch(Edges::from([])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				}),
			},
			Test {
				description: "target enabled with tags #1",
				input: (
					Config {
						targets: Some(TargetMap::from([(
							"foo".into(),
							Target {
								tags: Some(Tags {
									all_of: Some(TagSet::from(["test".into()])),
									..Tags::default()
								}),
								..Target::default()
							},
						)])),
						..Config::default()
					},
					(TagSet::from(["test".into()]), HashSet::from([])),
				),
				output: Ok(Tree {
					root: Node::Branch(Edges::from([("foo".into(), Node::Leaf("foo".into()))])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				}),
			},
			Test {
				description: "target enabled with tags #2",
				input: (
					Config {
						targets: Some(TargetMap::from([(
							"foo".into(),
							Target {
								tags: Some(Tags {
									all_of: Some(TagSet::from(["test".into()])),
									any_of: Some(TagSet::from(["foo".into(), "bar".into()])),
								}),
								..Target::default()
							},
						)])),
						..Config::default()
					},
					(
						TagSet::from(["test".into(), "bar".into()]),
						HashSet::from([]),
					),
				),
				output: Ok(Tree {
					root: Node::Branch(Edges::from([("foo".into(), Node::Leaf("foo".into()))])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				}),
			},
			Test {
				description: "target disabled due to disjunctive tags",
				input: (
					Config {
						targets: Some(TargetMap::from([(
							"foo".into(),
							Target {
								tags: Some(Tags {
									all_of: Some(TagSet::from(["test".into()])),
									any_of: Some(TagSet::from(["foo".into(), "bar".into()])),
								}),
								..Target::default()
							},
						)])),
						..Config::default()
					},
					(TagSet::from(["test".into()]), HashSet::from([])),
				),
				output: Ok(Tree {
					root: Node::Branch(Edges::from([])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				}),
			},
			Test {
				description: "target enabled with tags #3",
				input: (
					Config {
						targets: Some(TargetMap::from([(
							"foo".into(),
							Target {
								tags: Some(Tags {
									any_of: Some(TagSet::from(["test".into()])),
									..Tags::default()
								}),
								..Target::default()
							},
						)])),
						..Config::default()
					},
					(TagSet::from(["test".into()]), HashSet::from([])),
				),
				output: Ok(Tree {
					root: Node::Branch(Edges::from([("foo".into(), Node::Leaf("foo".into()))])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				}),
			},
			Test {
				description: "target using its file name as link name",
				input: (
					Config {
						targets: Some(TargetMap::from([("foo/bar/baz".into(), Target::default())])),
						..Config::default()
					},
					(TagSet::from([]), HashSet::from([])),
				),
				output: Ok(Tree {
					root: Node::Branch(Edges::from([(
						"foo".into(),
						Node::Branch(Edges::from([(
							"bar".into(),
							Node::Branch(Edges::from([("baz".into(), Node::Leaf("baz".into()))])),
						)])),
					)])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				}),
			},
			Test {
				description: "target enabled with target name filtering",
				input: (
					Config {
						targets: Some(TargetMap::from([
							("foo/bar".into(), Target::default()),
							("baz/qux".into(), Target::default()),
						])),
						..Config::default()
					},
					(TagSet::from([]), HashSet::from(["foo/bar".into()])),
				),
				output: Ok(Tree {
					root: Node::Branch(Edges::from([(
						"foo".into(),
						Node::Branch(Edges::from([("bar".into(), Node::Leaf("bar".into()))])),
					)])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				}),
			},
		]);

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

		let current_dir = &env::current_dir()?;

		let test_cases = Vec::from([
			Test {
				description: "single target should be ready",
				input: Tree {
					root: Node::Branch(Edges::from([(
						"tests/foo".into(),
						Node::Leaf("tests/foo".into()),
					)])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				},
				output: Tree {
					root: Node::Branch(Edges::from([(
						"tests/foo".into(),
						Node::Leaf("tests/foo".into()),
					)])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([("tests/foo".into(), Status::Ready)]),
				},
			},
			Test {
				description: "single target whose parent is nonexistent should be unparented",
				input: Tree {
					root: Node::Branch(Edges::from([(
						"xxx/foo".into(),
						Node::Leaf("xxx/foo".into()),
					)])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				},
				output: Tree {
					root: Node::Branch(Edges::from([(
						"xxx/foo".into(),
						Node::Leaf("xxx/foo".into()),
					)])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([("xxx/foo".into(), Status::Unparented)]),
				},
			},
			Test {
				description: "single target whose base directory is empty",
				input: Tree {
					root: Node::Branch(Edges::from([("foo".into(), Node::Leaf("foo".into()))])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				},
				output: Tree {
					root: Node::Branch(Edges::from([("foo".into(), Node::Leaf("foo".into()))])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([("foo".into(), Status::Ready)]),
				},
			},
			Test {
				description: "single target has conflict",
				input: Tree {
					root: Node::Branch(Edges::from([(
						"README.adoc".into(),
						Node::Leaf("README.adoc".into()),
					)])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				},
				output: Tree {
					root: Node::Branch(Edges::from([(
						"README.adoc".into(),
						Node::Leaf("README.adoc".into()),
					)])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([("README.adoc".into(), Status::Conflict)]),
				},
			},
			Test {
				description: "single target with wrong existing link",
				input: Tree {
					root: Node::Branch(Edges::from([(
						"something".into(),
						Node::Leaf("tests/data/something".into()),
					)])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([]),
				},
				output: Tree {
					root: Node::Branch(Edges::from([(
						"something".into(),
						Node::Leaf("tests/data/something".into()),
					)])),
					work_dir: current_dir.into(),
					statuses: Statuses::from([("tests/data/something".into(), Status::Mismatch)]),
				},
			},
			Test {
				description: "single target with correct existing link",
				input: Tree {
					root: Node::Branch(Edges::from([(
						"something".into(),
						Node::Leaf("tests/data/something".into()),
					)])),
					work_dir: "test".into(),
					statuses: Statuses::from([]),
				},
				output: Tree {
					root: Node::Branch(Edges::from([(
						"something".into(),
						Node::Leaf("tests/data/something".into()),
					)])),
					work_dir: "test".into(),
					statuses: Statuses::from([("tests/data/something".into(), Status::Done)]),
				},
			},
			Test {
				description: "link with invalid parent directory",
				input: Tree {
					root: Node::Branch(Edges::from([(
						"something".into(),
						Node::Leaf("LICENSE/something".into()),
					)])),
					work_dir: "test".into(),
					statuses: Statuses::from([]),
				},
				output: Tree {
					root: Node::Branch(Edges::from([(
						"something".into(),
						Node::Leaf("LICENSE/something".into()),
					)])),
					work_dir: "test".into(),
					statuses: Statuses::from([("LICENSE/something".into(), Status::Obstructed)]),
				},
			},
		]);

		for mut case in test_cases {
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
	fn link() -> Result<(), IoError> {
		struct Test<'a> {
			description: &'a str,
			input: Tree,
			output: Result<(), Error>,
			files_created: Vec<PathBuf>,
			dirs_created: Vec<PathBuf>,
		}

		let test_cases = Vec::from([
			Test {
				description: "nothing to be done",
				input: Tree {
					root: Node::Branch(Edges::from([(
						"foo".into(),
						Node::Leaf("tests/data/foo".into()),
					)])),
					work_dir: "fake_path".into(),
					statuses: Statuses::from([("tests/data/foo".into(), Status::Done)]),
				},
				output: Ok(()),
				files_created: Vec::from([]),
				dirs_created: Vec::from([]),
			},
			Test {
				description: "simple link",
				input: Tree {
					root: Node::Branch(Edges::from([(
						"foo".into(),
						Node::Leaf("tests/data/foo".into()),
					)])),
					work_dir: "fake_path".into(),
					statuses: Statuses::from([("tests/data/foo".into(), Status::Ready)]),
				},
				output: Ok(()),
				files_created: Vec::from(["tests/data/foo".into()]),
				dirs_created: Vec::from([]),
			},
			Test {
				description: "simple unparented link",
				input: Tree {
					root: Node::Branch(Edges::from([(
						"foo".into(),
						Node::Leaf("tests/xxx/foo".into()),
					)])),
					work_dir: "fake_path".into(),
					statuses: Statuses::from([("tests/xxx/foo".into(), Status::Unparented)]),
				},
				output: Ok(()),
				files_created: Vec::from(["tests/xxx/foo".into()]),
				dirs_created: Vec::from(["tests/xxx".into()]),
			},
			Test {
				description: "multiple links",
				input: Tree {
					root: Node::Branch(Edges::from([
						("foo".into(), Node::Leaf("tests/data/foo".into())),
						("bar".into(), Node::Leaf("tests/data/bar".into())),
					])),
					work_dir: "fake_path".into(),
					statuses: Statuses::from([
						("tests/data/foo".into(), Status::Ready),
						("tests/data/bar".into(), Status::Ready),
					]),
				},
				output: Ok(()),
				files_created: Vec::from(["tests/data/foo".into(), "tests/data/bar".into()]),
				dirs_created: Vec::from([]),
			},
			Test {
				description: "bad link with conflict",
				input: Tree {
					root: Node::Branch(Edges::from([(
						"something".into(),
						Node::Leaf("tests/data/something".into()),
					)])),
					work_dir: "fake_path".into(),
					statuses: Statuses::from([("tests/data/something".into(), Status::Conflict)]),
				},
				output: Err(Error::InternalError("tests/data/something".into())),
				files_created: Vec::from([]),
				dirs_created: Vec::from([]),
			},
			Test {
				description: "bad link with obstruction",
				input: Tree {
					root: Node::Branch(Edges::from([(
						"something".into(),
						Node::Leaf("tests/data/something".into()),
					)])),
					work_dir: "fake_path".into(),
					statuses: Statuses::from([("tests/data/something".into(), Status::Obstructed)]),
				},
				output: Err(Error::InternalError("tests/data/something".into())),
				files_created: Vec::from([]),
				dirs_created: Vec::from([]),
			},
			Test {
				input: Tree {
					root: Node::Branch(Edges::from([(
						"something".into(),
						Node::Leaf("tests/data/something".into()),
					)])),
					work_dir: "fake_path".into(),
					statuses: Statuses::from([("tests/data/something".into(), Status::Mismatch)]),
				},
				description: "bad link with mismatch",
				output: Err(Error::InternalError("tests/data/something".into())),
				files_created: Vec::from([]),
				dirs_created: Vec::from([]),
			},
		]);

		for case in test_cases {
			let got = case.input.link();

			let mut file_assertions = Vec::from([]);
			let mut dir_assertions = Vec::from([]);
			for file in &case.files_created {
				let link_path = PathBuf::from("fake_path").join(file.file_name().unwrap());

				file_assertions.push((file.read_link()?, link_path));

				fs::remove_file(file)?;
			}

			for dir in &case.dirs_created {
				dir_assertions.push((dir.is_dir(), dir));

				fs::remove_dir_all(dir)?;
			}

			for (got, want) in file_assertions {
				assert_eq!(
					got, want,
					"wrong target path for {:?} in {}",
					want, case.description
				);
			}

			for (is_dir, dir_path) in dir_assertions {
				assert!(
					is_dir,
					"did not create dir at {:?} in {}",
					dir_path, case.description
				);
			}

			assert_eq!(got, case.output, "bad result for {:?}", case.description);
		}

		Ok(())
	}
}
