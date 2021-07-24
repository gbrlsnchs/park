use std::{ffi::OsStr, path::PathBuf};

#[derive(Debug, PartialEq)]
pub enum Node {
	Root(Vec<Node>),
	Branch { path: PathBuf, children: Vec<Node> },
	Leaf { path: PathBuf },
}

impl Node {
	/// Adds a path to the node if and only if a node for that path doesn't exist yet.
	pub fn add(&mut self, path: PathBuf) {
		// Let's break the path into segments.
		let segments = path.iter().collect::<Vec<&OsStr>>();

		// Now let's isolate the first segment, which is the new node's key.
		if let Some((segment, rest)) = segments.split_first() {
			let segment = *segment;

			// We only support adding new nodes to nodes that are not leaves!
			match self {
				Self::Root(children) | Self::Branch { children, .. } => {
					// Let's check whether there's already a node with same name, otherwise let's
					// just create it.
					let child = children.iter_mut().find(|node| node.get_path() == segment);
					let node_exists = child.is_some();

					if rest.is_empty() {
						if node_exists {
							// TODO(gbrlsnchs): Handle this with an error maybe?
							return;
						}

						children.push(Self::Leaf {
							path: segment.into(),
						})
					} else {
						let rest = rest.iter().collect();
						if let Some(branch) = child {
							branch.add(rest);
						} else {
							let mut branch = Node::Branch {
								path: segment.into(),
								children: Vec::new(),
							};
							branch.add(rest);
							children.push(branch);
						}
					}
				}
				_ => {
					unimplemented!();
				}
			};
		}
	}

	/// Returns the segment path for the node. Root panics.
	fn get_path(&self) -> PathBuf {
		match self {
			Self::Leaf { path } => path.into(),
			Self::Branch { path, children: _ } => path.into(),
			_ => panic!("Can't get path for root node."),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	use pretty_assertions::assert_eq;

	#[test]
	fn test_add() {
		struct Test {
			node: Node,
			input: PathBuf,
			want: Node,
		}

		let test_cases = vec![
			Test {
				node: Node::Root(Vec::new()),
				input: PathBuf::from("foo"),
				want: Node::Root(vec![Node::Leaf {
					path: PathBuf::from("foo"),
				}]),
			},
			Test {
				node: Node::Root(Vec::new()),
				input: PathBuf::from("foo/bar"),
				want: Node::Root(vec![Node::Branch {
					path: PathBuf::from("foo"),
					children: vec![Node::Leaf {
						path: PathBuf::from("bar"),
					}],
				}]),
			},
		];

		for mut case in test_cases.into_iter() {
			case.node.add(case.input);
			assert_eq!(case.want, case.node);
		}
	}
}
