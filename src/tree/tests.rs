use std::{fs, path::PathBuf};

use indexmap::indexmap;
use indoc::indoc;
use maplit::{hashmap, hashset};
use pretty_assertions::assert_eq;

use crate::{
	config::{Link, Tags},
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

	let test_cases = vec![
		Test {
			description: "simple config with a single target",
			input: (
				Config {
					targets: Some(hashmap! {
						"foo".into() => Target::default()
					}),
					..Config::default()
				},
				hashset! {},
			),
			output: Ok(Tree {
				root: Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("foo".into()),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {},
			}),
		},
		Test {
			description: "simple config with a nested target",
			input: (
				Config {
					targets: Some(hashmap! {
						"foo/bar".into() => Target::default()
					}),
					..Config::default()
				},
				hashset! {},
			),
			output: Ok(Tree {
				root: Node::Branch(indexmap! {
					"foo".into() => Node::Branch(indexmap!{
						"bar".into() => Node::Leaf("bar".into()),
					}),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {},
			}),
		},
		Test {
			description: "target with custom options",
			input: (
				Config {
					targets: Some(hashmap! {
						"foo".into() => Target{
							link: Some(Link{
								name: Some("new_name".into()),
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
				root: Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("new_name".into()),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {},
			}),
		},
		Test {
			description: "target disabled due to conjunctive tags",
			input: (
				Config {
					targets: Some(hashmap! {
						"foo".into() => Target{
							tags: Some(Tags{
								all_of: Some(hashset!{"test".into()}),
								any_of: Some(hashset!{"foo/bar".into()}),
							}),
							..Target::default()
						},
					}),
					..Config::default()
				},
				hashset! {
					"foo".into(),
					"bar".into(),
				},
			),
			output: Ok(Tree {
				root: Node::Branch(indexmap! {}),
				work_dir: current_dir.into(),
				statuses: hashmap! {},
			}),
		},
		Test {
			description: "target enabled with tags #1",
			input: (
				Config {
					targets: Some(hashmap! {
						"foo".into() => Target{
							tags: Some(Tags{
								all_of: Some(hashset!{"test".into()}),
								..Tags::default()
							}),
							..Target::default()
						},
					}),
					..Config::default()
				},
				hashset! {
					"test".into(),
				},
			),
			output: Ok(Tree {
				root: Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("foo".into()),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {},
			}),
		},
		Test {
			description: "target enabled with tags #2",
			input: (
				Config {
					targets: Some(hashmap! {
						"foo".into() => Target{
							tags: Some(Tags{
								all_of: Some(hashset!{"test".into()}),
								any_of: Some(hashset!{"foo".into(), "bar".into()}),
							}),
							..Target::default()
						},
					}),
					..Config::default()
				},
				hashset! {
					"test".into(),
					"bar".into(),
				},
			),
			output: Ok(Tree {
				root: Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("foo".into()),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {},
			}),
		},
		Test {
			description: "target disabled due to disjunctive tags",
			input: (
				Config {
					targets: Some(hashmap! {
						"foo".into() => Target{
							tags: Some(Tags{
								all_of: Some(hashset!{"test".into()}),
								any_of: Some(hashset!{"foo".into(), "bar".into()}),
							}),
							..Target::default()
						},
					}),
					..Config::default()
				},
				hashset! {
					"test".into(),
				},
			),
			output: Ok(Tree {
				root: Node::Branch(indexmap! {}),
				work_dir: current_dir.into(),
				statuses: hashmap! {},
			}),
		},
		Test {
			description: "target enabled with tags #3",
			input: (
				Config {
					targets: Some(hashmap! {
						"foo".into() => Target{
							tags: Some(Tags{
								any_of: Some(hashset!{"test".into()}),
								..Tags::default()
							}),
							..Target::default()
						},
					}),
					..Config::default()
				},
				hashset! {
					"test".into(),
				},
			),
			output: Ok(Tree {
				root: Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("foo".into()),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {},
			}),
		},
		Test {
			description: "target using its file name as link name",
			input: (
				Config {
					targets: Some(hashmap! {
						"foo/bar/baz".into() => Target::default(),
					}),
					..Config::default()
				},
				hashset! {},
			),
			output: Ok(Tree {
				root: Node::Branch(indexmap! {
					"foo".into() => Node::Branch(indexmap!{
						"bar".into() => Node::Branch(indexmap!{
							"baz".into() => Node::Leaf("baz".into()),
						}),
					}),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {},
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

	let current_dir = &env::current_dir()?;

	let test_cases = vec![
		Test {
			description: "single target should be ready",
			input: Tree {
				root: Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("foo".into()),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {},
			},
			output: Tree {
				root: Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("foo".into()),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {
					"foo".into() => Status::Ready,
				},
			},
		},
		Test {
			description: "single target has conflict",
			input: Tree {
				root: Node::Branch(indexmap! {
					"README.md".into() => Node::Leaf("README.md".into()),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {},
			},
			output: Tree {
				root: Node::Branch(indexmap! {
					"README.md".into() => Node::Leaf("README.md".into()),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {
					"README.md".into() => Status::Conflict,
				},
			},
		},
		Test {
			description: "single target with wrong existing link",
			input: Tree {
				root: Node::Branch(indexmap! {
					"something".into() => Node::Leaf("tests/data/something".into()),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {},
			},
			output: Tree {
				root: Node::Branch(indexmap! {
					"something".into() => Node::Leaf("tests/data/something".into()),
				}),
				work_dir: current_dir.into(),
				statuses: hashmap! {
					"tests/data/something".into() => Status::Mismatch,
				},
			},
		},
		Test {
			description: "single target with correct existing link",
			input: Tree {
				root: Node::Branch(indexmap! {
					"something".into() => Node::Leaf("tests/data/something".into()),
				}),
				work_dir: "test".into(),
				statuses: hashmap! {},
			},
			output: Tree {
				root: Node::Branch(indexmap! {
					"something".into() => Node::Leaf("tests/data/something".into()),
				}),
				work_dir: "test".into(),
				statuses: hashmap! {
					"tests/data/something".into() => Status::Done,
				},
			},
		},
		Test {
			description: "link with invalid parent directory",
			input: Tree {
				root: Node::Branch(indexmap! {
					"something".into() => Node::Leaf("LICENSE/something".into()),
				}),
				work_dir: "test".into(),
				statuses: hashmap! {},
			},
			output: Tree {
				root: Node::Branch(indexmap! {
					"something".into() => Node::Leaf("LICENSE/something".into()),
				}),
				work_dir: "test".into(),
				statuses: hashmap! {
					"LICENSE/something".into() => Status::Obstructed,
				},
			},
		},
	];

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

	let test_cases = vec![
		Test {
			description: "nothing to be done",
			input: Tree {
				root: Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("tests/data/foo".into()),
				}),
				work_dir: "fake_path".into(),
				statuses: hashmap! {
					"tests/data/foo".into() => Status::Done,
				},
			},
			output: Ok(()),
			files_created: vec![],
			dirs_created: vec![],
		},
		Test {
			description: "simple link",
			input: Tree {
				root: Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("tests/data/foo".into()),
				}),
				work_dir: "fake_path".into(),
				statuses: hashmap! {
					"tests/data/foo".into() => Status::Ready,
				},
			},
			output: Ok(()),
			files_created: vec!["tests/data/foo".into()],
			dirs_created: vec![],
		},
		Test {
			description: "multiple links",
			input: Tree {
				root: Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("tests/data/foo".into()),
					"bar".into() => Node::Leaf("tests/data/bar".into()),
				}),
				work_dir: "fake_path".into(),
				statuses: hashmap! {
					"tests/data/foo".into() => Status::Ready,
					"tests/data/bar".into() => Status::Ready,
				},
			},
			output: Ok(()),
			files_created: vec!["tests/data/foo".into(), "tests/data/bar".into()],
			dirs_created: vec![],
		},
		Test {
			description: "bad link with conflict",
			input: Tree {
				root: Node::Branch(indexmap! {
					"something".into() => Node::Leaf("tests/data/something".into())
				}),
				work_dir: "fake_path".into(),
				statuses: hashmap! {
					"tests/data/something".into() => Status::Conflict,
				},
			},
			output: Err(Error::InternalError("tests/data/something".into())),
			files_created: vec![],
			dirs_created: vec![],
		},
		Test {
			description: "bad link with obstruction",
			input: Tree {
				root: Node::Branch(indexmap! {
					"something".into() => Node::Leaf("tests/data/something".into()),
				}),
				work_dir: "fake_path".into(),
				statuses: hashmap! {
					"tests/data/something".into() => Status::Obstructed,
				},
			},
			output: Err(Error::InternalError("tests/data/something".into())),
			files_created: vec![],
			dirs_created: vec![],
		},
		Test {
			input: Tree {
				root: Node::Branch(indexmap! {
					"something".into() => Node::Leaf("tests/data/something".into())
				}),
				work_dir: "fake_path".into(),
				statuses: hashmap! {
					"tests/data/something".into() => Status::Mismatch,
				},
			},
			description: "bad link with mismatch",
			output: Err(Error::InternalError("tests/data/something".into())),
			files_created: vec![],
			dirs_created: vec![],
		},
	];

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

#[test]
fn format_tree() -> Result<(), IoError> {
	let tree = Tree {
		root: Node::Branch(indexmap! {
			"foo".into() => Node::Branch(indexmap!{
				"bar".into() => Node::Leaf("bar".into()),
			}),
			"baz".into() => Node::Branch(indexmap!{
				"qux".into() => Node::Leaf("test/qux".into()),
			}),
			"quux".into() => Node::Branch(indexmap!{
				"quuz".into() => Node::Leaf("quuz".into()),
			}),
			"corge".into() => Node::Branch(indexmap!{
				"something".into() => Node::Leaf("tests/data/something".into()),
				"gralt".into() => Node::Leaf("test/gralt".into()),
				"anything".into() => Node::Leaf("file/anything".into()),
			}),
		}),
		statuses: hashmap! {
			"bar".into() => Status::Unknown,
			"test/qux".into() => Status::Done,
			"quuz".into() => Status::Ready,
			"tests/data/something".into() => Status::Mismatch,
			"test/gralt".into() => Status::Conflict,
			"file/anything".into() => Status::Obstructed,
		},
		work_dir: "test".into(),
	};

	println!("\n{}", tree);

	let link_color = Colour::Purple.normal();
	let symbols_color = Colour::White.dimmed();

	// TODO(gbrlsnchs): This can (and should) get better in the future. =)
	assert_eq!(
		tree.to_string(),
		format!(
			indoc! {"
					.                 {equals} {current_dir}
					{t_bar}foo                                   
					{straight_bar}{l_bar}bar       {arrow} {bar}                  {unknown}
					{t_bar}baz                                   
					{straight_bar}{l_bar}qux       {arrow} {test_qux}             {done}
					{t_bar}quux                                  
					{straight_bar}{l_bar}quuz      {arrow} {quuz}                 {ready}
					{l_bar}corge                                 
					{blank}{t_bar}something {arrow} {tests_data_something} {mismatch}
					{blank}{t_bar}gralt     {arrow} {test_gralt}           {conflict}
					{blank}{l_bar}anything  {arrow} {file_anything}        {obstructed}
				"},
			t_bar = symbols_color.paint("├── "),
			l_bar = symbols_color.paint("└── "),
			straight_bar = symbols_color.paint("│   "),
			blank = symbols_color.paint("    "),
			equals = symbols_color.paint(":="),
			arrow = symbols_color.paint("<-"),
			current_dir = Colour::Cyan.paint("test"),
			bar = link_color.paint("bar"),
			test_qux = link_color.paint("test/qux"),
			quuz = link_color.paint("quuz"),
			tests_data_something = link_color.paint("tests/data/something"),
			test_gralt = link_color.paint("test/gralt"),
			file_anything = link_color.paint("file/anything"),
			unknown = Colour::White.dimmed().bold().paint("(UNKNOWN)"),
			done = Colour::Blue.bold().paint("(DONE)"),
			ready = Colour::Green.bold().paint("(READY)"),
			mismatch = Colour::Yellow.bold().paint("(MISMATCH)"),
			conflict = Colour::Red.bold().paint("(CONFLICT)"),
			obstructed = Colour::Red.bold().paint("(OBSTRUCTED)"),
		)
	);

	Ok(())
}
