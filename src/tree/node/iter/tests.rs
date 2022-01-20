use indexmap::indexmap;
use pretty_assertions::assert_eq;

use super::*;

#[test]
fn iterate_in_correct_order() {
	let root = Node::Branch(indexmap! {
		"baz".into() => Node::Branch(indexmap!{
			"qux".into() => Node::Leaf("test/quxlinkku".into()),
		}),
		"test".into() => Node::Leaf("something/else".into()),
		"foo".into() => Node::Branch(indexmap!{
			"bar".into() => Node::Leaf("test/barlinkku".into()),
		}),
	});
	let mut iter = Iter {
		stack: vec![State {
			node: &root,
			segment: None,
			metadata: NodeMetadata {
				level: 0,
				last_sibling: false,
			},
		}],
		path_stack: Vec::new(),
	};

	assert_eq!(
		iter.next(),
		Some(Element {
			metadata: NodeMetadata {
				level: 0,
				last_sibling: false
			},
			target_path: "".into(),
			link_path: None,
		}),
	);
	assert_eq!(
		iter.next(),
		Some(Element {
			metadata: NodeMetadata {
				level: 1,
				last_sibling: false
			},
			target_path: "baz".into(),
			link_path: None,
		}),
	);
	assert_eq!(
		iter.next(),
		Some(Element {
			metadata: NodeMetadata {
				level: 2,
				last_sibling: true
			},
			target_path: "baz/qux".into(),
			link_path: Some("test/quxlinkku".into()),
		}),
	);
	assert_eq!(
		iter.next(),
		Some(Element {
			metadata: NodeMetadata {
				level: 1,
				last_sibling: false
			},
			target_path: "test".into(),
			link_path: Some("something/else".into()),
		}),
	);
	assert_eq!(
		iter.next(),
		Some(Element {
			metadata: NodeMetadata {
				level: 1,
				last_sibling: true
			},
			target_path: "foo".into(),
			link_path: None,
		}),
	);
	assert_eq!(
		iter.next(),
		Some(Element {
			metadata: NodeMetadata {
				level: 2,
				last_sibling: true
			},
			target_path: "foo/bar".into(),
			link_path: Some("test/barlinkku".into()),
		}),
	);
	assert_eq!(iter.next(), None);
}
