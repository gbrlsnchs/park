use std::{fs, path::PathBuf};

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
		Test {
			description: "link with invalid parent directory",
			input: Tree {
				root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
					link_path: PathBuf::from("LICENSE/something"),
					target_path: PathBuf::from("something"),
					status: Status::Unknown,
				})])),
				work_dir: PathBuf::from("test"),
			},
			output: Tree {
				root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
					link_path: PathBuf::from("LICENSE/something"),
					target_path: PathBuf::from("something"),
					status: Status::Obstructed,
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
fn link() -> Result<(), IoError> {
	struct Test<'a> {
		description: &'a str,
		input: Tree,
		output: Result<Vec<PathBuf>, Error>,
	}

	let test_cases = vec![
		Test {
			input: Tree {
				root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
					target_path: PathBuf::from("foo"),
					link_path: PathBuf::from("tests/data/foo"),
					status: Status::Done,
				})])),
				work_dir: PathBuf::from("fake_path"),
			},
			description: "nothing to be done",
			output: Ok(vec![]),
		},
		Test {
			input: Tree {
				root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
					target_path: PathBuf::from("foo"),
					link_path: PathBuf::from("tests/data/foo"),
					status: Status::Ready,
				})])),
				work_dir: PathBuf::from("fake_path"),
			},
			description: "simple link",
			output: Ok(vec![PathBuf::from("tests/data/foo")]),
		},
		Test {
			input: Tree {
				root: Node::new_ref(Node::Root(vec![
					Node::new_ref(Node::Leaf {
						target_path: PathBuf::from("foo"),
						link_path: PathBuf::from("tests/data/foo"),
						status: Status::Ready,
					}),
					Node::new_ref(Node::Leaf {
						target_path: PathBuf::from("bar"),
						link_path: PathBuf::from("tests/data/bar"),
						status: Status::Ready,
					}),
				])),
				work_dir: PathBuf::from("fake_path"),
			},
			description: "multiple links",
			output: Ok(vec![
				PathBuf::from("tests/data/foo"),
				PathBuf::from("tests/data/bar"),
			]),
		},
		Test {
			input: Tree {
				root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
					target_path: PathBuf::from("something"),
					link_path: PathBuf::from("tests/data/something"),
					status: Status::Conflict,
				})])),
				work_dir: PathBuf::from("fake_path"),
			},
			description: "bad link with conflict",
			output: Err(Error::InternalError(PathBuf::from("tests/data/something"))),
		},
		Test {
			input: Tree {
				root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
					target_path: PathBuf::from("something"),
					link_path: PathBuf::from("tests/data/something"),
					status: Status::Obstructed,
				})])),
				work_dir: PathBuf::from("fake_path"),
			},
			description: "bad link with obstruction",
			output: Err(Error::InternalError(PathBuf::from("tests/data/something"))),
		},
		Test {
			input: Tree {
				root: Node::new_ref(Node::Root(vec![Node::new_ref(Node::Leaf {
					target_path: PathBuf::from("something"),
					link_path: PathBuf::from("tests/data/something"),
					status: Status::Mismatch,
				})])),
				work_dir: PathBuf::from("fake_path"),
			},
			description: "bad link with mismatch",
			output: Err(Error::InternalError(PathBuf::from("tests/data/something"))),
		},
	];

	for case in test_cases {
		let got = case.input.link();

		if let Ok(links) = &got {
			let mut assertions = Vec::new();

			for link in links {
				assertions.push((
					link.read_link()?,
					PathBuf::from("fake_path").join(link.file_name().unwrap()),
				));

				fs::remove_file(link)?;
			}

			for (new_target_path, link_path) in assertions {
				assert_eq!(
					new_target_path, link_path,
					"wrong target path for {:?}",
					case.description,
				);
			}
		}

		assert_eq!(got, case.output, "bad result for {:?}", case.description);
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
					link_path: PathBuf::from("test/qux"),
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
				children: vec![
					Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("tests/data/something"),
						target_path: PathBuf::from("something"),
						status: Status::Mismatch,
					}),
					Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("test/gralt"),
						target_path: PathBuf::from("corge/gralt"),
						status: Status::Conflict,
					}),
					Node::new_ref(Node::Leaf {
						link_path: PathBuf::from("file/anything"),
						target_path: PathBuf::from("anything"),
						status: Status::Obstructed,
					}),
				],
			}),
		])),
		work_dir: PathBuf::from("test"),
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
