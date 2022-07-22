use std::{fs, path::PathBuf};

use pretty_assertions::assert_eq;

use crate::{
	config::{Link, TagSet, Tags, TargetMap},
	tree::node::{error::Error as NodeError, Status},
};

use super::*;

#[test]
fn parse() -> Result<(), IoError> {
	struct Test<'a> {
		description: &'a str,
		input: (Config, TagSet),
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
				TagSet::new(),
			),
			output: Ok(Tree {
				root: Node::Branch(Edges::from([("foo".into(), Node::Leaf("foo".into()))])),
				work_dir: current_dir.into(),
				statuses: Statuses::new(),
			}),
		},
		Test {
			description: "simple config with a nested target",
			input: (
				Config {
					targets: Some(TargetMap::from([("foo/bar".into(), Target::default())])),
					..Config::default()
				},
				TagSet::new(),
			),
			output: Ok(Tree {
				root: Node::Branch(Edges::from([(
					"foo".into(),
					Node::Branch(Edges::from([("bar".into(), Node::Leaf("bar".into()))])),
				)])),
				work_dir: current_dir.into(),
				statuses: Statuses::new(),
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
				TagSet::new(),
			),
			output: Ok(Tree {
				root: Node::Branch(Edges::from([("foo".into(), Node::Leaf("new_name".into()))])),
				work_dir: current_dir.into(),
				statuses: Statuses::new(),
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
				TagSet::from(["foo".into(), "bar".into()]),
			),
			output: Ok(Tree {
				root: Node::Branch(Edges::new()),
				work_dir: current_dir.into(),
				statuses: Statuses::new(),
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
				TagSet::from(["test".into()]),
			),
			output: Ok(Tree {
				root: Node::Branch(Edges::from([("foo".into(), Node::Leaf("foo".into()))])),
				work_dir: current_dir.into(),
				statuses: Statuses::new(),
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
				TagSet::from(["test".into(), "bar".into()]),
			),
			output: Ok(Tree {
				root: Node::Branch(Edges::from([("foo".into(), Node::Leaf("foo".into()))])),
				work_dir: current_dir.into(),
				statuses: Statuses::new(),
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
				TagSet::from(["test".into()]),
			),
			output: Ok(Tree {
				root: Node::Branch(Edges::new()),
				work_dir: current_dir.into(),
				statuses: Statuses::new(),
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
				TagSet::from(["test".into()]),
			),
			output: Ok(Tree {
				root: Node::Branch(Edges::from([("foo".into(), Node::Leaf("foo".into()))])),
				work_dir: current_dir.into(),
				statuses: Statuses::new(),
			}),
		},
		Test {
			description: "target using its file name as link name",
			input: (
				Config {
					targets: Some(TargetMap::from([("foo/bar/baz".into(), Target::default())])),
					..Config::default()
				},
				TagSet::new(),
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
				statuses: Statuses::new(),
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
				root: Node::Branch(Edges::from([("foo".into(), Node::Leaf("foo".into()))])),
				work_dir: current_dir.into(),
				statuses: Statuses::new(),
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
					"README.md".into(),
					Node::Leaf("README.md".into()),
				)])),
				work_dir: current_dir.into(),
				statuses: Statuses::new(),
			},
			output: Tree {
				root: Node::Branch(Edges::from([(
					"README.md".into(),
					Node::Leaf("README.md".into()),
				)])),
				work_dir: current_dir.into(),
				statuses: Statuses::from([("README.md".into(), Status::Conflict)]),
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
				statuses: Statuses::new(),
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
				statuses: Statuses::new(),
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
				statuses: Statuses::new(),
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
			files_created: Vec::new(),
			dirs_created: Vec::new(),
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
			dirs_created: Vec::new(),
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
			dirs_created: Vec::new(),
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
			files_created: Vec::new(),
			dirs_created: Vec::new(),
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
			files_created: Vec::new(),
			dirs_created: Vec::new(),
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
			files_created: Vec::new(),
			dirs_created: Vec::new(),
		},
	]);

	for case in test_cases {
		let got = case.input.link();

		let mut file_assertions = Vec::new();
		let mut dir_assertions = Vec::new();
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
