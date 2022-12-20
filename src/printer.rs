use std::{
	ffi::{OsStr, OsString},
	fmt::{Display, Error as FmtError, Formatter, Result as FmtResult},
	io::Write,
	str,
};

use ansi_term::{Colour, Style};
use tabwriter::TabWriter;

use crate::parser::{
	iter::{Element as IterElement, NodeMetadata},
	node::Status,
	tree::Tree,
};

pub struct Printer<'a> {
	pub tree: &'a Tree,
	pub colored: bool,
	pub home: Option<OsString>,
}

impl<'a> Printer<'a> {
	fn resolve_style(&self, style: Style) -> Style {
		if self.colored {
			style
		} else {
			Style::new()
		}
	}

	fn replace_home<S>(&self, s: S) -> String
	where
		S: AsRef<str>,
	{
		if let Some(home) = self.home.as_ref().and_then(|s| s.to_str()) {
			return s.as_ref().replacen(home, "~", 1);
		}

		s.as_ref().into()
	}
}

impl<'a> Display for Printer<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let table = Vec::new();
		let mut tab_writer = TabWriter::new(table).padding(1);

		let mut indent_blocks = Vec::<bool>::new();

		for IterElement {
			metadata: NodeMetadata {
				last_sibling: last_edge,
				level,
			},
			target_path,
			link_path,
		} in &self.tree.root
		{
			if level == 0 {
				let cwd = self.resolve_style(Colour::White.italic()).paint({
					let path = self.replace_home(self.tree.work_dir.to_string_lossy());

					if self.colored {
						path
					} else {
						format!("({})", path)
					}
				});

				if writeln!(tab_writer, ". {}", cwd,).is_err() {
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

				if write!(
					tab_writer,
					"{}",
					self.resolve_style(Colour::White.normal()).paint(segment)
				)
				.is_err()
				{
					return Err(FmtError);
				}
			}

			if let Some(link_path) = link_path {
				let default_status = Status::Unknown;
				let status = self
					.tree
					.statuses
					.get(&link_path)
					.unwrap_or(&default_status);

				let status_style = self.resolve_style(
					match status {
						Status::Unknown => Colour::White,
						Status::Done => Colour::Blue,
						Status::Ready => Colour::Green,
						Status::Mismatch | Status::Unparented => Colour::Yellow,
						Status::Conflict | Status::Obstructed => Colour::Red,
					}
					.reverse(),
				);
				let status = if self.colored {
					format!(" {:?} ", status)
				} else {
					format!("[{:?}]", status)
				}
				.to_uppercase();

				let target_segment: Vec<&OsStr> = target_path.iter().collect();
				let is_leaf = level == target_segment.len();
				let target_path = target_path.file_name().unwrap().to_string_lossy();
				if writeln!(
					tab_writer,
					"{target_path}\t{link_path}\t{status}",
					target_path = {
						let mut style = Style::new();

						if is_leaf {
							style = Colour::Cyan.bold();
						}

						self.resolve_style(style).paint(target_path)
					},
					link_path = self.resolve_style(Colour::Purple.italic()).paint({
						let path = self.replace_home(link_path.to_string_lossy());

						if self.colored {
							format!(" {} ", path)
						} else {
							format!("({})", path)
						}
					}),
					status = status_style.paint(status),
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

#[cfg(test)]
mod tests {
	use std::io::Error as IoError;

	use indoc::indoc;
	use pretty_assertions::assert_eq;

	use crate::parser::{
		node::{Edges, Node},
		tree::Statuses,
	};

	use super::*;

	#[test]
	fn format_tree() -> Result<(), IoError> {
		let tree = Tree {
			root: Node::Branch(Edges::from([
				(
					"baz".into(),
					Node::Branch(Edges::from([("qux".into(), Node::Leaf("test/qux".into()))])),
				),
				(
					"corge".into(),
					Node::Branch(Edges::from([
						("anything".into(), Node::Leaf("file/file".into())),
						("gralt".into(), Node::Leaf("test/gralt".into())),
						(
							"something".into(),
							Node::Leaf("tests/data/something".into()),
						),
						(
							"s0m37h1ng".into(),
							Node::Leaf("tests/none/s0m37h1ng".into()),
						),
					])),
				),
				(
					"foo".into(),
					Node::Branch(Edges::from([("bar".into(), Node::Leaf("bar".into()))])),
				),
				(
					"quux".into(),
					Node::Branch(Edges::from([("quuz".into(), Node::Leaf("quuz".into()))])),
				),
			])),
			statuses: Statuses::from([
				("bar".into(), Status::Unknown),
				("test/qux".into(), Status::Done),
				("quuz".into(), Status::Ready),
				("tests/data/something".into(), Status::Mismatch),
				("tests/none/s0m37h1ng".into(), Status::Unparented),
				("test/gralt".into(), Status::Conflict),
				("file/file".into(), Status::Obstructed),
			]),
			work_dir: "test".into(),
		};

		{
			let printer = Printer {
				tree: &tree,
				colored: true,
				home: Some("file".into()),
			};

			println!("\n{}", printer);

			let target_color = Colour::Cyan.bold();
			let link_color = Colour::Purple.italic();
			let symbols_color = Colour::White.normal();

			assert_eq!(
				printer.to_string(),
				format!(
					indoc! {"
					. {current_dir}
					{t_bar}baz                                  
					{straight_bar}{l_bar}{tgt1}       {test_qux}             {done}
					{t_bar}corge                                
					{straight_bar}{t_bar}{tgt2}  {file_file}               {obstructed}
					{straight_bar}{t_bar}{tgt3}     {test_gralt}           {conflict}
					{straight_bar}{t_bar}{tgt4} {tests_data_something} {mismatch}
					{straight_bar}{l_bar}{tgt5} {tests_none_something} {unparented}
					{t_bar}foo                                  
					{straight_bar}{l_bar}{tgt6}       {bar}                  {unknown}
					{l_bar}quux                                 
					{blank}{l_bar}{tgt7}      {quuz}                 {ready}
				"},
					t_bar = symbols_color.paint("├── "),
					l_bar = symbols_color.paint("└── "),
					tgt1 = target_color.paint("qux"),
					tgt2 = target_color.paint("anything"),
					tgt3 = target_color.paint("gralt"),
					tgt4 = target_color.paint("something"),
					tgt5 = target_color.paint("s0m37h1ng"),
					tgt6 = target_color.paint("bar"),
					tgt7 = target_color.paint("quuz"),
					straight_bar = symbols_color.paint("│   "),
					blank = symbols_color.paint("    "),
					current_dir = Colour::White.italic().paint("test"),
					bar = link_color.paint(" bar "),
					test_qux = link_color.paint(" test/qux "),
					quuz = link_color.paint(" quuz "),
					tests_data_something = link_color.paint(" tests/data/something "),
					tests_none_something = link_color.paint(" tests/none/s0m37h1ng "),
					test_gralt = link_color.paint(" test/gralt "),
					file_file = link_color.paint(" ~/file "),
					unknown = Colour::White.reverse().paint(" UNKNOWN "),
					done = Colour::Blue.reverse().paint(" DONE "),
					ready = Colour::Green.reverse().paint(" READY "),
					unparented = Colour::Yellow.reverse().paint(" UNPARENTED "),
					mismatch = Colour::Yellow.reverse().paint(" MISMATCH "),
					conflict = Colour::Red.reverse().paint(" CONFLICT "),
					obstructed = Colour::Red.reverse().paint(" OBSTRUCTED "),
				),
				"invalid colored output",
			);
		}

		{
			let printer = Printer {
				tree: &tree,
				colored: false,
				home: Some("file".into()),
			};

			println!("\n{}", printer);

			assert_eq!(
				printer.to_string(),
				format!(indoc! {"
					. (test)
					├── baz                                  
					│   └── qux       (test/qux)             [DONE]
					├── corge                                
					│   ├── anything  (~/file)               [OBSTRUCTED]
					│   ├── gralt     (test/gralt)           [CONFLICT]
					│   ├── something (tests/data/something) [MISMATCH]
					│   └── s0m37h1ng (tests/none/s0m37h1ng) [UNPARENTED]
					├── foo                                  
					│   └── bar       (bar)                  [UNKNOWN]
					└── quux                                 
					    └── quuz      (quuz)                 [READY]
				"}),
				"invalid non-colored output",
			);
		}

		Ok(())
	}
}
