use pretty_assertions::assert_eq;

use super::*;

#[test]
fn test_add() {
	let default_base_dir = PathBuf::from("default_base_dir");

	struct Test<'a> {
		description: &'a str,
		node_before: Node,
		input: (PathBuf, Link),
		node_after: Node,
		want: AddResult,
	}

	let test_cases = vec![
		Test {
			description: "simple first node",
			node_before: Node::Root(Vec::new()),
			input: (PathBuf::from("foo"), Link::default()),
			node_after: Node::Root(vec![Node::new_ref(Node::Leaf {
				link_path: default_base_dir.join("foo"),
				target_path: PathBuf::from("foo"),
				status: Status::Unknown,
			})]),
			want: Ok(()),
		},
		Test {
			description: "simple nested node",
			node_before: Node::Root(Vec::new()),
			input: (PathBuf::from("foo/bar"), Link::default()),
			node_after: Node::Root(vec![Node::new_ref(Node::Branch {
				path: PathBuf::from("foo"),
				children: vec![Node::new_ref(Node::Leaf {
					link_path: default_base_dir.join("bar"),
					target_path: PathBuf::from("foo/bar"),
					status: Status::Unknown,
				})],
			})]),
			want: Ok(()),
		},
		Test {
			description: "simple node to existing branch",
			node_before: Node::Root(vec![Node::new_ref(Node::Branch {
				path: PathBuf::from("foo"),
				children: vec![Node::new_ref(Node::Leaf {
					link_path: default_base_dir.join("bar"),
					target_path: PathBuf::from("foo/bar"),
					status: Status::Unknown,
				})],
			})]),
			input: (PathBuf::from("foo/test"), Link::default()),
			node_after: Node::Root(vec![Node::new_ref(Node::Branch {
				path: PathBuf::from("foo"),
				children: vec![
					Node::new_ref(Node::Leaf {
						link_path: default_base_dir.join("bar"),
						target_path: PathBuf::from("foo/bar"),
						status: Status::Unknown,
					}),
					Node::new_ref(Node::Leaf {
						link_path: default_base_dir.join("test"),
						target_path: PathBuf::from("foo/test"),
						status: Status::Unknown,
					}),
				],
			})]),
			want: Ok(()),
		},
		Test {
			description: "leaf exists for simple node",
			node_before: Node::Root(vec![Node::new_ref(Node::Leaf {
				link_path: default_base_dir.join("foo"),
				target_path: PathBuf::from("foo"),
				status: Status::Unknown,
			})]),
			input: (PathBuf::from("foo"), Link::default()),
			node_after: Node::Root(vec![Node::new_ref(Node::Leaf {
				link_path: default_base_dir.join("foo"),
				target_path: PathBuf::from("foo"),
				status: Status::Unknown,
			})]),
			want: Err(AddError::LeafExists(PathBuf::from("foo"))),
		},
		Test {
			description: "leaf exists for nested node",
			node_before: Node::Root(vec![Node::new_ref(Node::Branch {
				path: PathBuf::from("foo"),
				children: vec![Node::new_ref(Node::Leaf {
					link_path: default_base_dir.join("bar"),
					target_path: PathBuf::from("bar"),
					status: Status::Unknown,
				})],
			})]),
			input: (PathBuf::from("foo"), Link::default()),
			node_after: Node::Root(vec![Node::new_ref(Node::Branch {
				path: PathBuf::from("foo"),
				children: vec![Node::new_ref(Node::Leaf {
					link_path: default_base_dir.join("bar"),
					target_path: PathBuf::from("bar"),
					status: Status::Unknown,
				})],
			})]),
			want: Err(AddError::LeafExists(PathBuf::from("foo"))),
		},
		Test {
			description: "new link name for simple first node",
			node_before: Node::Root(Vec::new()),
			input: (
				PathBuf::from("foo"),
				Link {
					name: Some(PathBuf::from("new_name")),
					..Link::default()
				},
			),
			node_after: Node::Root(vec![Node::new_ref(Node::Leaf {
				link_path: default_base_dir.join("new_name"),
				target_path: PathBuf::from("foo"),
				status: Status::Unknown,
			})]),
			want: Ok(()),
		},
		Test {
			description: "new link name for nested node",
			node_before: Node::Root(Vec::new()),
			input: (
				PathBuf::from("foo/bar"),
				Link {
					name: Some(PathBuf::from("new_name")),
					..Link::default()
				},
			),
			node_after: Node::Root(vec![Node::new_ref(Node::Branch {
				path: PathBuf::from("foo"),
				children: vec![Node::new_ref(Node::Leaf {
					link_path: default_base_dir.join("new_name"),
					target_path: PathBuf::from("foo/bar"),
					status: Status::Unknown,
				})],
			})]),
			want: Ok(()),
		},
		Test {
			description: "empty link name for simple first node",
			node_before: Node::Root(Vec::new()),
			input: (
				PathBuf::from("foo"),
				Link {
					name: Some(PathBuf::new()),
					..Link::default()
				},
			),
			node_after: Node::Root(vec![Node::new_ref(Node::Leaf {
				link_path: default_base_dir.join("foo"),
				target_path: PathBuf::from("foo"),
				status: Status::Unknown,
			})]),
			want: Ok(()),
		},
		Test {
			description: "empty link name for nested node",
			node_before: Node::Root(Vec::new()),
			input: (
				PathBuf::from("foo/bar"),
				Link {
					name: Some(PathBuf::new()),
					..Link::default()
				},
			),
			node_after: Node::Root(vec![Node::new_ref(Node::Branch {
				path: PathBuf::from("foo"),
				children: vec![Node::new_ref(Node::Leaf {
					link_path: default_base_dir.join("bar"),
					target_path: PathBuf::from("foo/bar"),
					status: Status::Unknown,
				})],
			})]),
			want: Ok(()),
		},
		Test {
			description: "different base directory for simple first node",
			node_before: Node::Root(Vec::new()),
			input: (
				PathBuf::from("foo"),
				Link {
					base_dir: Some(PathBuf::from("alt_base_dir")),
					..Link::default()
				},
			),
			node_after: Node::Root(vec![Node::new_ref(Node::Leaf {
				link_path: PathBuf::from("alt_base_dir").join("foo"),
				target_path: PathBuf::from("foo"),
				status: Status::Unknown,
			})]),
			want: Ok(()),
		},
		Test {
			description: "empty link name for nested node with alternative base directory",
			node_before: Node::Root(Vec::new()),
			input: (
				PathBuf::from("foo/bar"),
				Link {
					base_dir: Some(PathBuf::from("alt_base_dir")),
					..Link::default()
				},
			),
			node_after: Node::Root(vec![Node::new_ref(Node::Branch {
				path: PathBuf::from("foo"),
				children: vec![Node::new_ref(Node::Leaf {
					link_path: PathBuf::from("alt_base_dir").join("bar"),
					target_path: PathBuf::from("foo/bar"),
					status: Status::Unknown,
				})],
			})]),
			want: Ok(()),
		},
	];

	for mut case in test_cases {
		let got = case
			.node_before
			.add(&default_base_dir, case.input.0, case.input.1);

		assert_eq!(got, case.want, "bad result for {:?}", case.description);
		assert_eq!(
			case.node_before, case.node_after,
			"nodes mismatch for {:?}",
			case.description
		);
	}
}

#[test]
fn test_error_messages() {
	let test_cases = vec![
		(
			AddError::LeafAsBranch(PathBuf::from("foo/bar")),
			r#"node for "foo/bar" is leaf, not branch"#,
		),
		(
			AddError::LeafExists(PathBuf::from("foo/bar")),
			r#"leaf already exists for "foo/bar""#,
		),
	];

	for case in test_cases {
		let got = case.0.to_string();

		assert_eq!(got, case.1);
	}
}
