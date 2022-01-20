use std::error::Error;
use std::ffi::OsString;
use std::io::Write;
use std::result::Result as StdResult;

use clap::Parser;

use crate::config::Config;
use crate::tree::Tree;

pub type Result = StdResult<(), Box<dyn Error>>;

#[derive(Default, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
	#[clap(long, short, help = "Try to link eligible targets")]
	link: bool,

	#[clap(parse(from_os_str), help = "List of additional tags")]
	tags: Vec<OsString>,
}

/// Runs the program, parsing STDIN for a config file.
pub fn run(input: &str, mut stdout: impl Write, args: Args) -> Result {
	let config: Config = toml::from_str(input)?;

	let mut tree = Tree::parse(
		config,
		args.tags
			.iter()
			.map(|tag| tag.to_string_lossy().into())
			.collect(),
	)?;

	tree.analyze()?;

	if args.link {
		let _results = tree.link()?;
	} else {
		write!(stdout, "{}", tree)?;
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use std::env;

	use ansi_term::Colour;
	use indoc::indoc;
	use pretty_assertions::assert_eq;

	use super::*;

	#[test]
	fn test_running_without_args() -> Result {
		let input = indoc! {r#"
			base_dir = "tests"

			[targets.foo]
			[targets.bar]
		"#};
		let mut stdout = Vec::new();

		run(input, &mut stdout, Args::default())?;

		let link_color = Colour::Purple.normal();
		let symbols_color = Colour::White.dimmed();
		let current_dir = env::current_dir().unwrap_or_default();

		assert_eq!(
			String::from_utf8(stdout).unwrap(),
			format!(
				indoc! {"
				.       {equals} {current_dir}
				{t_bar}bar {arrow} {bar} {ready}
				{l_bar}foo {arrow} {foo} {ready}
			"},
				t_bar = symbols_color.paint("├── "),
				l_bar = symbols_color.paint("└── "),
				equals = symbols_color.paint(":="),
				arrow = symbols_color.paint("<-"),
				current_dir = Colour::Cyan.paint(current_dir.to_string_lossy()),
				foo = link_color.paint("tests/foo"),
				bar = link_color.paint("tests/bar"),
				ready = Colour::Green.bold().paint("(READY)"),
			)
		);

		Ok(())
	}

	#[test]
	fn test_running_with_tags_as_args() -> Result {
		let input = indoc! {r#"
			base_dir = "tests"

			[targets.foo]
			tags.all_of = ["foo"]

			[targets.bar]
		"#};
		let mut stdout = Vec::new();

		run(
			input,
			&mut stdout,
			Args {
				tags: vec!["foo".into()],
				..Args::default()
			},
		)?;

		let link_color = Colour::Purple.normal();
		let symbols_color = Colour::White.dimmed();
		let current_dir = env::current_dir().unwrap_or_default();

		assert_eq!(
			String::from_utf8(stdout).unwrap(),
			format!(
				indoc! {"
				.       {equals} {current_dir}
				{t_bar}bar {arrow} {bar} {ready}
				{l_bar}foo {arrow} {foo} {ready}
			"},
				t_bar = symbols_color.paint("├── "),
				l_bar = symbols_color.paint("└── "),
				equals = symbols_color.paint(":="),
				arrow = symbols_color.paint("<-"),
				current_dir = Colour::Cyan.paint(current_dir.to_string_lossy()),
				foo = link_color.paint("tests/foo"),
				bar = link_color.paint("tests/bar"),
				ready = Colour::Green.bold().paint("(READY)"),
			)
		);

		Ok(())
	}
}
