use std::path::PathBuf;
use std::{ffi::OsString, io::Write};

use anyhow::{Context, Result};

use crate::cli::Park;
use crate::parser::tree::LinkOpts;
use crate::{config::Config, parser::tree::Tree, printer::Printer};

pub struct Env {
	pub colored: bool,
	pub home: Option<OsString>,
}

/// Runs the program, parsing STDIN for a config file.
pub fn run<W>(env: Env, input: &str, mut stdout: W, cli: Park) -> Result<()>
where
	W: Write,
{
	let config: Config =
		toml::from_str(input).with_context(|| "could not read input configuration")?;
	let Park {
		link,
		filters,
		replace,
		create_dirs,
	} = cli;

	let (tags, targets): (Vec<String>, Vec<String>) =
		filters.into_iter().partition(|s| s.starts_with('+'));

	let tags = tags.iter().map(|s| &s[1..]).map(|s| s.into()).collect();
	let targets = targets.iter().map(PathBuf::from).collect();

	let mut tree = Tree::parse(
		config,
		(tags, targets),
		LinkOpts {
			replace,
			create_dirs,
		},
	)
	.with_context(|| "could not parse target")?;

	tree.analyze()
		.with_context(|| "could not analyze targets")?;

	if link {
		tree.link().with_context(|| "could not link targets")?;
	} else {
		write!(
			stdout,
			"{}",
			Printer {
				tree: &tree,
				colored: env.colored,
				home: env.home,
			}
		)
		.with_context(|| "could not print preview tree")?;
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
	fn test_running_without_args() -> Result<()> {
		let input = indoc! {r#"
			base_dir = "tests"

			[targets.0xDEADBABE]
			tags.all_of = ["0xDEADBABE"]

			[targets.0xDEADBEEF]
		"#};

		{
			let mut stdout = Vec::new();

			run(
				Env {
					colored: true,
					home: None,
				},
				input,
				&mut stdout,
				Park::default(),
			)?;

			let target_color = Colour::Cyan.bold();
			let link_color = Colour::Purple.italic();
			let symbols_color = Colour::White.normal();
			let current_dir = env::current_dir().unwrap_or_default();

			assert_eq!(
				String::from_utf8(stdout).unwrap(),
				format!(
					indoc! {"
						. {current_dir}
						{l_bar}{tgt} {beef} {ready}
					"},
					tgt = target_color.paint("0xDEADBEEF"),
					l_bar = symbols_color.paint("└── "),
					current_dir = Colour::White.italic().paint(current_dir.to_string_lossy()),
					beef = link_color.paint(" tests/0xDEADBEEF "),
					ready = Colour::Green.reverse().paint(" READY "),
				),
				"with color",
			);
		}

		{
			let mut stdout = Vec::new();

			run(
				Env {
					colored: false,
					home: None,
				},
				&input,
				&mut stdout,
				Park {
					filters: vec!["+0xDEADBABE".into()],
					..Park::default()
				},
			)?;

			let current_dir = env::current_dir().unwrap_or_default();

			assert_eq!(
				String::from_utf8(stdout).unwrap(),
				format!(
					indoc! {"
						. ({current_dir})
						├── 0xDEADBABE (tests/0xDEADBABE) [READY]
						└── 0xDEADBEEF (tests/0xDEADBEEF) [READY]
					"},
					current_dir = current_dir.to_string_lossy(),
				),
				"without color",
			);
		}

		Ok(())
	}

	#[test]
	fn test_running_with_tags_as_args() -> Result<()> {
		let input = indoc! {r#"
			base_dir = "tests"

			[targets.foo]
			tags.all_of = ["foo"]

			[targets.bar]
		"#};

		let mut stdout = Vec::new();

		run(
			Env {
				colored: true,
				home: None,
			},
			&input,
			&mut stdout,
			Park {
				filters: vec!["+foo".into()],
				..Park::default()
			},
		)?;

		let target_color = Colour::Cyan.bold();
		let link_color = Colour::Purple.italic();
		let symbols_color = Colour::White.normal();
		let current_dir = env::current_dir().unwrap_or_default();

		assert_eq!(
			String::from_utf8(stdout).unwrap(),
			format!(
				indoc! {"
						. {current_dir}
						{t_bar}{tgt1} {bar} {ready}
						{l_bar}{tgt2} {foo} {ready}
					"},
				tgt1 = target_color.paint("bar"),
				tgt2 = target_color.paint("foo"),
				t_bar = symbols_color.paint("├── "),
				l_bar = symbols_color.paint("└── "),
				current_dir = Colour::White.italic().paint(current_dir.to_string_lossy()),
				foo = link_color.paint(" tests/foo "),
				bar = link_color.paint(" tests/bar "),
				ready = Colour::Green.reverse().paint(" READY "),
			),
			"invalid colored output",
		);

		Ok(())
	}

	#[test]
	fn test_running_with_target_filters_as_args() -> Result<()> {
		let input = indoc! {r#"
			base_dir = "tests"

			[targets.foo]

			[targets.bar]
		"#};

		let mut stdout = Vec::new();

		env::remove_var("NO_COLOR");
		run(
			Env {
				colored: true,
				home: None,
			},
			&input,
			&mut stdout,
			Park {
				filters: vec!["foo".into()],
				..Park::default()
			},
		)?;

		let link_color = Colour::Purple.italic();
		let target_color = Colour::Cyan.bold();
		let symbols_color = Colour::White.normal();
		let current_dir = env::current_dir().unwrap_or_default();

		assert_eq!(
			String::from_utf8(stdout).unwrap(),
			format!(
				indoc! {"
						. {current_dir}
						{l_bar}{tgt} {foo} {ready}
					"},
				l_bar = symbols_color.paint("└── "),
				current_dir = Colour::White.italic().paint(current_dir.to_string_lossy()),
				tgt = target_color.paint("foo"),
				foo = link_color.paint(" tests/foo "),
				ready = Colour::Green.reverse().paint(" READY "),
			),
			"invalid colored output",
		);

		Ok(())
	}

	#[test]
	fn test_linking() -> Result<()> {
		let input = indoc! {r#"
			base_dir = "tests"

			[targets.skip_me]
			tags.all_of = ["dont_skip"]

			[targets.my_symlink]
		"#};
		let mut stdout = Vec::new();

		run(
			Env {
				colored: true,
				home: None,
			},
			input,
			&mut stdout,
			Park {
				link: true,
				..Park::default()
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
