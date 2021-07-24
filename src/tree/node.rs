use std::path::PathBuf;

pub enum Node {
	Root(Vec<Node>),
	Branch { path: PathBuf, children: Vec<Node> },
	Leaf { path: PathBuf },
}
