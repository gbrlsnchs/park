use pretty_assertions::assert_eq;

use crate::tree::node::Edges;

use super::*;

#[test]
fn iterate_in_correct_order() {
	let root = Node::Branch(Edges::from([
		(
			"baz".into(),
			Node::Branch(Edges::from([("qux".into(), Node::Leaf("test/qux".into()))])),
		),
		(
			"foo".into(),
			Node::Branch(Edges::from([("bar".into(), Node::Leaf("test/bar".into()))])),
		),
		("test".into(), Node::Leaf("something/else".into())),
	]));
	let mut iter = Iter {
		stack: Vec::from([State {
			node: &root,
			segment: None,
			metadata: NodeMetadata {
				level: 0,
				last_sibling: false,
			},
		}]),
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
			link_path: Some("test/qux".into()),
		}),
	);
	assert_eq!(
		iter.next(),
		Some(Element {
			metadata: NodeMetadata {
				level: 1,
				last_sibling: false
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
			link_path: Some("test/bar".into()),
		}),
	);
	assert_eq!(
		iter.next(),
		Some(Element {
			metadata: NodeMetadata {
				level: 1,
				last_sibling: true
			},
			target_path: "test".into(),
			link_path: Some("something/else".into()),
		}),
	);
	assert_eq!(iter.next(), None);
}
