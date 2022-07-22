use std::{collections::HashMap, env, io::Error as IoError, os::unix::fs, path::PathBuf};

use crate::config::{Config, TagSet, Tags, Target};

use self::{
	error::Error,
	node::{error::Error as NodeError, iter::Element as IterElement, Edges, Node, Status},
};

pub type Statuses = HashMap<PathBuf, Status>;

/// Structure representing all dotfiles after reading a configuration for Park.
#[derive(Debug, PartialEq)]
pub struct Tree {
	pub root: Node,
	pub work_dir: PathBuf,
	pub statuses: Statuses,
}

impl<'a> Tree {
	/// Parses a configuration and returns a tree based on it.
	pub fn parse(config: Config, mut runtime_tags: TagSet) -> Result<Self, NodeError> {
		let targets = config.targets.unwrap_or_default();

		let cwd = env::current_dir().unwrap_or_default();
		let work_dir = config.work_dir.unwrap_or(cwd);

		let mut tree = Tree {
			root: Node::Branch(Edges::new()),
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
			let link_path = link.name.map_or_else(
				|| {
					target_path
						.file_name()
						.map(|file_name| base_dir.join(file_name))
						.unwrap()
				},
				|name| base_dir.join(name),
			);
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

		for IterElement {
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
			.filter(|IterElement { link_path, .. }| link_path.is_some()) // filters branches
			.filter(|IterElement { link_path, .. }| {
				if let Some(Status::Done) = self.statuses.get(link_path.as_ref().unwrap()) {
					return false;
				}

				true
			})
			.map(
				|IterElement {
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
							| Status::Obstructed => return Err(Error::InternalError(link_path)),
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
