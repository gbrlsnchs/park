use std::{
	cell::RefCell,
	env,
	fmt::{Display, Error as FmtError, Formatter, Result as FmtResult},
	io::{Error as IoError, Write},
	os::unix::fs,
	path::PathBuf,
	rc::Rc,
	str,
};

use ansi_term::Colour;
use tabwriter::TabWriter;

use crate::{
	config::{Config, TagSet, Tags, Target},
	tree::node::Status,
};

use self::{error::Error, iter::DepthFirstIter, node::NodeRef};
use self::{
	iter::NodeEntry,
	node::{AddError, Node},
};

mod error;
mod iter;
mod node;

#[cfg(test)]
mod tests;

/// Structure representing all dotfiles after reading a configuration for Park.
#[derive(Debug, PartialEq)]
pub struct Tree {
	root: NodeRef,
	work_dir: PathBuf,
}

impl<'a> Tree {
	/// Parses a configuration and returns a tree based on it.
	pub fn parse(config: Config, mut runtime_tags: TagSet) -> Result<Self, AddError> {
		let targets = config.targets.unwrap_or_default();

		let cwd = env::current_dir().unwrap_or_default();
		let work_dir = config.work_dir.unwrap_or(cwd);

		let tree = Tree {
			root: Rc::new(RefCell::new(Node::Root(Vec::with_capacity(targets.len())))),
			work_dir,
		};

		let Config {
			base_dir: ref default_base_dir,
			tags: default_tags,
			..
		} = config;

		if let Some(default_tags) = default_tags {
			runtime_tags.extend(default_tags);
		}

		'targets: for (target_path, target) in targets {
			let Target {
				link,
				tags: target_tags,
			} = target;

			let target_tags = target_tags.unwrap_or_default();

			let Tags { all_of, any_of } = target_tags;
			let (all_of, any_of) = (all_of.unwrap_or_default(), any_of.unwrap_or_default());

			let mut allowed = true;
			for tag in &all_of {
				allowed = allowed && runtime_tags.contains(tag);

				if !allowed {
					continue 'targets;
				}
			}

			// No disjunctive tags? Pass.
			let mut allowed = any_of.is_empty();
			for tag in &any_of {
				allowed = allowed || runtime_tags.contains(tag);
			}
			if !allowed {
				continue;
			}

			tree.root
				.borrow_mut()
				.add(default_base_dir, target_path, link.unwrap_or_default())?;
		}

		Ok(tree)
	}

	/// Analyze the tree's nodes in order to check viability for symlinks to be done.
	/// This means it will iterate the tree and update each node's status.
	pub fn analyze(&self) -> Result<(), IoError> {
		for NodeEntry { node_ref, .. } in self {
			let mut node = node_ref.borrow_mut();

			if let Node::Leaf {
				target_path,
				link_path,
				status,
			} = &mut *node
			{
				if let Some(parent_dir) = link_path.parent() {
					if parent_dir.exists() && !parent_dir.is_dir() {
						*status = Status::Obstructed;

						continue;
					}
				}

				let existing_target_path = link_path.read_link();

				if existing_target_path.is_err() {
					*status = if link_path.exists() {
						Status::Conflict
					} else {
						Status::Ready
					};

					continue;
				}

				let existing_target_path = existing_target_path.unwrap();

				let target_path = self.work_dir.join(target_path);

				*status = if existing_target_path == target_path {
					Status::Done
				} else {
					Status::Mismatch
				}
			}
		}

		Ok(())
	}

	pub fn link(&self) -> Result<Vec<PathBuf>, Error> {
		let links: Result<Vec<(PathBuf, PathBuf)>, Error> = self
			.into_iter()
			.filter(|NodeEntry { node_ref, .. }| {
				let node = node_ref.borrow();

				// Only leaves not already done should be linked.
				match &*node {
					Node::Leaf { status, .. } => *status != Status::Done,
					_ => false,
				}
			})
			.map(|NodeEntry { node_ref, .. }| {
				let node = node_ref.borrow();

				if let Node::Leaf {
					status,
					target_path,
					link_path,
				} = &*node
				{
					match status {
						// TODO: Return more detailed errors.
						Status::Mismatch | Status::Conflict | Status::Obstructed => {
							return Err(Error::InternalError(link_path.clone()))
						}
						_ => {}
					}

					return Ok((self.work_dir.join(target_path), link_path.clone()));
				}

				unreachable!();
			})
			.collect();

		let mut created_links = Vec::new();
		for (target_path, link_path) in links? {
			if let Err(err) = fs::symlink(target_path, &link_path) {
				return Err(Error::IoError(err.kind()));
			};

			created_links.push(link_path);
		}

		Ok(created_links)
	}
}

impl<'a> IntoIterator for &'a Tree {
	type Item = NodeEntry;
	type IntoIter = DepthFirstIter;

	fn into_iter(self) -> Self::IntoIter {
		DepthFirstIter::new(Rc::clone(&self.root))
	}
}

impl<'a> Display for Tree {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let table = Vec::new();
		let mut tab_writer = TabWriter::new(table).padding(1);

		let mut indent_blocks = Vec::<bool>::new();

		for NodeEntry {
			deepest,
			level,
			node_ref,
		} in self
		{
			let node = node_ref.borrow();

			if let Node::Root(..) = *node {
				let cwd = Colour::Cyan.paint(self.work_dir.to_string_lossy());
				if writeln!(
					tab_writer,
					".\t{} {}",
					Colour::White.dimmed().paint(":="),
					cwd,
				)
				.is_err()
				{
					return Err(FmtError);
				}

				continue;
			}

			while level <= indent_blocks.len() {
				indent_blocks.pop();
			}

			indent_blocks.push(deepest);

			for (idx, has_indent_guide) in indent_blocks.iter().enumerate() {
				let is_leaf = idx == level - 1;

				let segment = match (has_indent_guide, is_leaf) {
					(true, true) => "└── ",
					(false, true) => "├── ",
					(true, _) => "    ",
					(false, _) => "│   ",
				};

				if write!(tab_writer, "{}", Colour::White.dimmed().paint(segment)).is_err() {
					return Err(FmtError);
				}
			}

			match &*node {
				Node::Branch { path, .. } => {
					if writeln!(tab_writer, "{}\t\t", path.to_string_lossy()).is_err() {
						return Err(FmtError);
					};
				}
				Node::Leaf {
					target_path,
					link_path,
					status,
				} => {
					let status_style = match status {
						Status::Unknown => Colour::White.dimmed(),
						Status::Done => Colour::Blue.normal(),
						Status::Ready => Colour::Green.normal(),
						Status::Mismatch => Colour::Yellow.normal(),
						Status::Conflict | Status::Obstructed => Colour::Red.normal(),
					};
					let status = format!("({:?})", status).to_uppercase();

					if writeln!(
						tab_writer,
						"{target_path}\t{arrow} {link_path}\t{status}",
						target_path = target_path.file_name().unwrap().to_string_lossy(),
						arrow = Colour::White.dimmed().paint("<-"),
						link_path = Colour::Purple.paint(link_path.to_string_lossy()),
						status = status_style.bold().paint(status),
					)
					.is_err()
					{
						return Err(FmtError);
					};
				}
				_ => {}
			}
		}

		match tab_writer.into_inner() {
			Err(_) => return Err(FmtError),
			Ok(w) => {
				write!(f, "{}", str::from_utf8(&w).unwrap())?;
			}
		}

		Ok(())
	}
}
