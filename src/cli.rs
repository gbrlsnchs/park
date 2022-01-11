use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::result::Result as StdResult;

use clap::Parser;

use crate::config::Config;
use crate::tree::Tree;

pub type Result = StdResult<(), Box<dyn Error>>;

#[derive(Parser)]
pub struct Args {}

/// Runs the program, parsing STDIN for a config file.
pub fn run() -> Result {
	let mut input = String::new();

	let stdin = io::stdin();
	let mut handle = stdin.lock();
	handle.read_to_string(&mut input)?;

	let config: Config = toml::from_str(&input)?;

	let (runtime_tags, flags): (Vec<OsString>, Vec<OsString>) = env::args_os()
		.skip(1)
		.partition(|arg| arg.to_string_lossy().starts_with('+'));

	let _args = Args::parse_from(flags);

	let tree = Tree::parse(
		config,
		runtime_tags
			.iter()
			.map(|tag| tag.to_string_lossy().trim_start_matches('+').into())
			.filter(|tag: &String| !tag.is_empty())
			.collect(),
	)?;

	tree.analyze()?;

	let stdout = io::stdout();
	let mut handle = stdout.lock();

	write!(handle, "{}", tree)?;

	Ok(())
}
