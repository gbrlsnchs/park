use std::{
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
}

impl<'a> Printer<'a> {
	fn resolve_style(&self, style: Style) -> Style {
		if self.colored {
			style
		} else {
			Style::new()
		}
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
				let cwd = self
					.resolve_style(Colour::Cyan.normal())
					.paint(self.tree.work_dir.to_string_lossy());
				if writeln!(
					tab_writer,
					".\t{} {}",
					self.resolve_style(Colour::White.dimmed()).paint(":="),
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

				if write!(
					tab_writer,
					"{}",
					self.resolve_style(Colour::White.dimmed()).paint(segment)
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
						Status::Unknown => Colour::White.dimmed(),
						Status::Done => Colour::Blue.normal(),
						Status::Ready => Colour::Green.normal(),
						Status::Mismatch => Colour::Yellow.normal(),
						Status::Conflict | Status::Obstructed => Colour::Red.normal(),
					}
					.bold(),
				);
				let status = format!("({:?})", status).to_uppercase();

				if writeln!(
					tab_writer,
					"{target_path}\t{arrow} {link_path}\t{status}",
					target_path = target_path.file_name().unwrap().to_string_lossy(),
					arrow = self.resolve_style(Colour::White.dimmed()).paint("<-"),
					link_path = self
						.resolve_style(Colour::Purple.normal())
						.paint(link_path.to_string_lossy()),
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
					"foo".into(),
					Node::Branch(Edges::from([("bar".into(), Node::Leaf("bar".into()))])),
				),
				(
					"baz".into(),
					Node::Branch(Edges::from([("qux".into(), Node::Leaf("test/qux".into()))])),
				),
				(
					"quux".into(),
					Node::Branch(Edges::from([("quuz".into(), Node::Leaf("quuz".into()))])),
				),
				(
					"corge".into(),
					Node::Branch(Edges::from([
						(
							"something".into(),
							Node::Leaf("tests/data/something".into()),
						),
						("gralt".into(), Node::Leaf("test/gralt".into())),
						("anything".into(), Node::Leaf("file/anything".into())),
					])),
				),
			])),
			statuses: Statuses::from([
				("bar".into(), Status::Unknown),
				("test/qux".into(), Status::Done),
				("quuz".into(), Status::Ready),
				("tests/data/something".into(), Status::Mismatch),
				("test/gralt".into(), Status::Conflict),
				("file/anything".into(), Status::Obstructed),
			]),
			work_dir: "test".into(),
		};

		{
			let printer = Printer {
				tree: &tree,
				colored: true,
			};

			println!("\n{}", printer);

			let link_color = Colour::Purple.normal();
			let symbols_color = Colour::White.dimmed();

			assert_eq!(
				printer.to_string(),
				format!(
					indoc! {"
					.                 {equals} {current_dir}
					{t_bar}baz                                   
					{straight_bar}{l_bar}qux       {arrow} {test_qux}             {done}
					{t_bar}corge                                 
					{straight_bar}{t_bar}anything  {arrow} {file_anything}        {obstructed}
					{straight_bar}{t_bar}gralt     {arrow} {test_gralt}           {conflict}
					{straight_bar}{l_bar}something {arrow} {tests_data_something} {mismatch}
					{t_bar}foo                                   
					{straight_bar}{l_bar}bar       {arrow} {bar}                  {unknown}
					{l_bar}quux                                  
					{blank}{l_bar}quuz      {arrow} {quuz}                 {ready}
				"},
					t_bar = symbols_color.paint("├── "),
					l_bar = symbols_color.paint("└── "),
					straight_bar = symbols_color.paint("│   "),
					blank = symbols_color.paint("    "),
					equals = symbols_color.paint(":="),
					arrow = symbols_color.paint("<-"),
					current_dir = Colour::Cyan.paint("test"),
					bar = link_color.paint("bar"),
					test_qux = link_color.paint("test/qux"),
					quuz = link_color.paint("quuz"),
					tests_data_something = link_color.paint("tests/data/something"),
					test_gralt = link_color.paint("test/gralt"),
					file_anything = link_color.paint("file/anything"),
					unknown = Colour::White.dimmed().bold().paint("(UNKNOWN)"),
					done = Colour::Blue.bold().paint("(DONE)"),
					ready = Colour::Green.bold().paint("(READY)"),
					mismatch = Colour::Yellow.bold().paint("(MISMATCH)"),
					conflict = Colour::Red.bold().paint("(CONFLICT)"),
					obstructed = Colour::Red.bold().paint("(OBSTRUCTED)"),
				),
				"invalid colored output",
			);
		}

		{
			let printer = Printer {
				tree: &tree,
				colored: false,
			};

			println!("\n{}", printer);

			assert_eq!(
				printer.to_string(),
				format!(indoc! {"
					.                 := test
					├── baz                                   
					│   └── qux       <- test/qux             (DONE)
					├── corge                                 
					│   ├── anything  <- file/anything        (OBSTRUCTED)
					│   ├── gralt     <- test/gralt           (CONFLICT)
					│   └── something <- tests/data/something (MISMATCH)
					├── foo                                   
					│   └── bar       <- bar                  (UNKNOWN)
					└── quux                                  
					    └── quuz      <- quuz                 (READY)
				"}),
				"invalid non-colored output",
			);
		}

		Ok(())
	}
}
