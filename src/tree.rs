use std::{
	collections::HashMap,
	env,
	fmt::{Display, Error as FmtError, Formatter, Result as FmtResult},
	io::{Error as IoError, Write},
	os::unix::fs,
	path::PathBuf,
	str,
};

use ansi_term::Colour;
use indexmap::IndexMap;
use tabwriter::TabWriter;

use crate::config::{Config, TagSet, Tags, Target};

use self::{
	error::Error,
	iter::{NodeIterEntry, NodeMetadata},
	node::{error::Error as NodeError, Node, Status},
};

mod error;
mod iter;
mod node;

#[cfg(test)]
mod tests;

pub type Statuses = HashMap<PathBuf, Status>;

/// Structure representing all dotfiles after reading a configuration for Park.
#[derive(Debug, PartialEq)]
pub struct Tree {
	root: Node,
	work_dir: PathBuf,
	statuses: Statuses,
}

impl<'a> Tree {
	/// Parses a configuration and returns a tree based on it.
	pub fn parse(config: Config, mut runtime_tags: TagSet) -> Result<Self, NodeError> {
		let targets = config.targets.unwrap_or_default();

		let cwd = env::current_dir().unwrap_or_default();
		let work_dir = config.work_dir.unwrap_or(cwd);

		let mut tree = Tree {
			root: Node::Branch(IndexMap::new()),
			work_dir,
			statuses: Statuses::new(),
		};

		let Config {
			base_dir: default_base_dir,
			tags: default_tags,
			..
		} = config;

		if let Some(default_tags) = default_tags {
			runtime_tags.extend(default_tags);
		}

		'targets: for (ref target_path, target) in targets {
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

			let link = link.unwrap_or_default();
			let base_dir = link.base_dir.as_ref().unwrap_or(&default_base_dir);
			let link_path = link
				.name
				.map_or_else(|| base_dir.join(target_path), |name| base_dir.join(name));
			tree.root.add(target_path.iter().collect(), link_path)?;
		}

		Ok(tree)
	}

	/// Analyze the tree's nodes in order to check viability for symlinks to be done.
	/// This means it will iterate the tree and update each node's status.
	pub fn analyze(&mut self) -> Result<(), IoError> {
		let Tree {
			ref mut statuses,
			ref root,
			..
		} = self;

		for NodeIterEntry {
			link_path,
			target_path,
			..
		} in root
		{
			if let Some(link_path) = link_path {
				if let Some(parent_dir) = link_path.parent() {
					if parent_dir.exists() && !parent_dir.is_dir() {
						statuses.insert(link_path, Status::Obstructed);

						continue;
					}
				}

				let existing_target_path = link_path.read_link();

				if existing_target_path.is_err() {
					let link_exists = link_path.exists();
					statuses.insert(
						link_path,
						if link_exists {
							Status::Conflict
						} else {
							Status::Ready
						},
					);

					continue;
				}

				let existing_target_path = existing_target_path.unwrap();

				let target_path = self.work_dir.join(target_path);

				statuses.insert(
					link_path,
					if existing_target_path == target_path {
						Status::Done
					} else {
						Status::Mismatch
					},
				);
			}
		}

		Ok(())
	}

	pub fn link(&self) -> Result<(), Error> {
		let links: Result<Vec<(PathBuf, PathBuf)>, Error> = self
			.root
			.into_iter()
			.filter(|NodeIterEntry { link_path, .. }| link_path.is_some()) // filters branches
			.filter(|NodeIterEntry { link_path, .. }| {
				if let Some(Status::Done) = self.statuses.get(link_path.as_ref().unwrap()) {
					return false;
				}

				true
			})
			.map(
				|NodeIterEntry {
				     target_path,
				     link_path,
				     ..
				 }| {
					let link_path = link_path.unwrap();

					if let Some(status) = self.statuses.get(&link_path) {
						match status {
							// TODO: Return more detailed errors.
							Status::Unknown
							| Status::Mismatch
							| Status::Conflict
							| Status::Obstructed => return Err(Error::InternalError(link_path.clone())),
							_ => {}
						}

						return Ok((self.work_dir.join(target_path), link_path.clone()));
					}

					unreachable!();
				},
			)
			.collect();

		let mut created_links = Vec::new();
		for (target_path, link_path) in links? {
			if let Err(err) = fs::symlink(target_path, &link_path) {
				return Err(Error::IoError(err.kind()));
			};

			created_links.push(link_path);
		}

		Ok(())
	}
}

impl<'a> Display for Tree {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let table = Vec::new();
		let mut tab_writer = TabWriter::new(table).padding(1);

		let mut indent_blocks = Vec::<bool>::new();

		for NodeIterEntry {
			metadata: NodeMetadata { last_edge, level },
			target_path,
			link_path,
		} in &self.root
		{
			if level == 0 {
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

			indent_blocks.push(last_edge);

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

			if let Some(link_path) = link_path {
				let default_status = Status::Unknown;
				let status = self.statuses.get(&link_path).unwrap_or(&default_status);

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
			} else {
				let path = target_path.file_name().unwrap();
				if writeln!(tab_writer, "{}\t\t", path.to_string_lossy()).is_err() {
					return Err(FmtError);
				};
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
