use std::ffi::OsString;

use indexmap::indexmap;
use pretty_assertions::assert_eq;

use super::*;

#[test]
fn test_add_nodes() {
	struct Test<'a> {
		description: &'a str,
		input: (Node, Vec<&'a OsStr>, PathBuf),
		output: (Node, Result<(), Error>),
	}

	let foo = OsString::from("foo");
	let bar = OsString::from("bar");
	let baz = OsString::from("baz");
	let capital_e = OsString::from("E");

	let test_cases = vec![
		Test {
			description: "simple first node",
			input: (Node::Branch(indexmap! {}), vec![&foo], "test/foo".into()),
			output: (
				Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("test/foo".into()),
				}),
				Ok(()),
			),
		},
		Test {
			description: "add sibling node to existing one",
			input: (
				Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("test/foo".into()),
				}),
				vec![&bar],
				"yay/bar".into(),
			),
			output: (
				Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("test/foo".into()),
					"bar".into() => Node::Leaf("yay/bar".into()),
				}),
				Ok(()),
			),
		},
		Test {
			description: "add nested node",
			input: (
				Node::Branch(indexmap! {}),
				vec![&foo, &bar],
				"test/bar".into(),
			),
			output: (
				Node::Branch(indexmap! {
					"foo".into() => Node::Branch(indexmap!{
						"bar".into() => Node::Leaf("test/bar".into()),
					}),
				}),
				Ok(()),
			),
		},
		Test {
			description: "add sibling to nested node",
			input: (
				Node::Branch(indexmap! {
					"foo".into() => Node::Branch(indexmap!{
						"bar".into() => Node::Leaf("test/bar".into()),
					}),
				}),
				vec![&foo, &baz],
				"yay/baz".into(),
			),
			output: (
				Node::Branch(indexmap! {
					"foo".into() => Node::Branch(indexmap!{
						"bar".into() => Node::Leaf("test/bar".into()),
						"baz".into() => Node::Leaf("yay/baz".into()),
					}),
				}),
				Ok(()),
			),
		},
		Test {
			description: "add existing node path",
			input: (
				Node::Branch(indexmap! {
					"foo".into() => Node::Branch(indexmap!{
						"bar".into() => Node::Leaf("test/bar".into()),
					}),
				}),
				vec![&foo, &bar],
				"please/let_me_in".into(),
			),
			output: (
				Node::Branch(indexmap! {
					"foo".into() => Node::Branch(indexmap!{
						"bar".into() => Node::Leaf("test/bar".into()),
					}),
				}),
				Err(Error::LeafExists("bar".into(), "please/let_me_in".into())),
			),
		},
		Test {
			description: "add node to a leaf node",
			input: (
				Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("test/foo".into()),
				}),
				vec![&foo, &bar],
				"please/let_me_in".into(),
			),
			output: (
				Node::Branch(indexmap! {
					"foo".into() => Node::Leaf("test/foo".into()),
				}),
				Err(Error::NotABranch("bar".into(), "please/let_me_in".into())),
			),
		},
		Test {
			description: "add node to a leaf node",
			input: (
				Node::Branch(indexmap! {}),
				vec![],
				"please/let_me_in".into(),
			),
			output: (Node::Branch(indexmap! {}), Err(Error::EmptySegment)),
		},
		Test {
			description: "add nodes to get sorted",
			input: (
				Node::Branch(indexmap! {
					"C".into() => Node::Leaf("1".into()),
					"Z".into() => Node::Leaf("2".into()),
					"B".into() => Node::Leaf("3".into()),
					"A".into() => Node::Leaf("4".into()),
				}),
				vec![&capital_e],
				"5".into(),
			),
			output: (
				Node::Branch(indexmap! {
					"A".into() => Node::Leaf("4".into()),
					"B".into() => Node::Leaf("3".into()),
					"C".into() => Node::Leaf("1".into()),
					"E".into() => Node::Leaf("5".into()),
					"Z".into() => Node::Leaf("2".into()),
				}),
				Ok(()),
			),
		},
	];

	for case in test_cases {
		let (mut tree, segments, link) = case.input;

		let result = tree.add(segments, link);
		let (want_tree, want_result) = case.output;

		assert_eq!(
			tree, want_tree,
			"mismatch when adding nodes: {:?}",
			case.description
		);
		assert_eq!(result, want_result);
	}
}
