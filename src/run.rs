use std::io::Write;
use std::path::PathBuf;
use std::result::Result as StdResult;
use std::{env, error::Error};

use crate::{cli::Args, config::Config, parser::tree::Tree, printer::Printer};

pub type Result = StdResult<(), Box<dyn Error>>;

/// Runs the program, parsing STDIN for a config file.
pub fn run(input: &str, mut stdout: impl Write, args: Args) -> Result {
	let config: Config = toml::from_str(input)?;
	let Args { link, filters } = args;

	let (tags, targets): (Vec<String>, Vec<String>) =
		filters.into_iter().partition(|s| s.starts_with('+'));

	let tags = tags.iter().map(|s| &s[1..]).map(|s| s.into()).collect();
	let targets = targets.iter().map(PathBuf::from).collect();

	let mut tree = Tree::parse(config, (tags, targets))?;

	tree.analyze()?;

	if link {
		let _results = tree.link()?;
	} else {
		write!(
			stdout,
			"{}",
			Printer {
				tree: &tree,
				colored: match env::var("NO_COLOR") {
					Ok(_) => false,
					Err(_) => true,
				}
			}
		)?;
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use std::{env, fs, path::PathBuf, str};

	use ansi_term::Colour;
	use indoc::indoc;
	use pretty_assertions::assert_eq;

	use super::*;

	#[test]
	fn test_running_without_args() -> Result {
		let input = indoc! {r#"
			base_dir = "tests"

			[targets.0xDEADBABE]
			tags.all_of = ["0xDEADBABE"]

			[targets.0xDEADBEEF]
		"#};

		{
			let mut stdout = Vec::new();

			run(input, &mut stdout, Args::default())?;

			let link_color = Colour::Purple.normal();
			let symbols_color = Colour::White.dimmed();
			let current_dir = env::current_dir().unwrap_or_default();

			assert_eq!(
				String::from_utf8(stdout).unwrap(),
				format!(
					indoc! {"
						.              {equals} {current_dir}
						{l_bar}0xDEADBEEF {arrow} {beef} {ready}
					"},
					l_bar = symbols_color.paint("└── "),
					equals = symbols_color.paint(":="),
					arrow = symbols_color.paint("<-"),
					current_dir = Colour::Cyan.paint(current_dir.to_string_lossy()),
					beef = link_color.paint("tests/0xDEADBEEF"),
					ready = Colour::Green.bold().paint("(READY)"),
				),
				"with color",
			);
		}

		{
			let mut stdout = Vec::new();

			env::set_var("NO_COLOR", "1");
			run(
				&input,
				&mut stdout,
				Args {
					filters: vec!["+0xDEADBABE".into()],
					..Args::default()
				},
			)?;
			env::remove_var("NO_COLOR");

			let current_dir = env::current_dir().unwrap_or_default();

			assert_eq!(
				String::from_utf8(stdout).unwrap(),
				format!(
					indoc! {"
						.              := {current_dir}
						├── 0xDEADBABE <- tests/0xDEADBABE (READY)
						└── 0xDEADBEEF <- tests/0xDEADBEEF (READY)
					"},
					current_dir = current_dir.to_string_lossy(),
				),
				"without color",
			);
		}

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
			&input,
			&mut stdout,
			Args {
				filters: vec!["+foo".into()],
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
			),
			"invalid colored output",
		);

		Ok(())
	}

	#[test]
	fn test_running_with_target_filters_as_args() -> Result {
		let input = indoc! {r#"
			base_dir = "tests"

			[targets.foo]

			[targets.bar]
		"#};

		let mut stdout = Vec::new();

		run(
			&input,
			&mut stdout,
			Args {
				filters: vec!["foo".into()],
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
						{l_bar}foo {arrow} {foo} {ready}
					"},
				l_bar = symbols_color.paint("└── "),
				equals = symbols_color.paint(":="),
				arrow = symbols_color.paint("<-"),
				current_dir = Colour::Cyan.paint(current_dir.to_string_lossy()),
				foo = link_color.paint("tests/foo"),
				ready = Colour::Green.bold().paint("(READY)"),
			),
			"invalid colored output",
		);

		Ok(())
	}

	#[test]
	fn test_linking() -> Result {
		let input = indoc! {r#"
			base_dir = "tests"

			[targets.skip_me]
			tags.all_of = ["dont_skip"]

			[targets.my_symlink]
		"#};
		let mut stdout = Vec::new();

		run(
			input,
			&mut stdout,
			Args {
				link: true,
				..Args::default()
			},
		)?;

		let link_path = "tests/my_symlink";
		let link = PathBuf::from(link_path).read_link();
		fs::remove_file(link_path)?;

		assert!(link.is_ok());
		assert_eq!(str::from_utf8(&stdout).unwrap(), "");

		Ok(())
	}
}
