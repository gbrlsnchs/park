use std::error::Error;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::result::Result as StdResult;

use clap::Parser;

use crate::config::Config;
use crate::tree::Tree;

pub type Result = StdResult<(), Box<dyn Error>>;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
	#[clap(long, short, help = "Try to link eligible targets")]
	link: bool,

	#[clap(parse(from_os_str), help = "List of additional tags")]
	tags: Vec<OsString>,
}

/// Runs the program, parsing STDIN for a config file.
pub fn run() -> Result {
	let mut input = String::new();

	let stdin = io::stdin();
	let mut handle = stdin.lock();
	handle.read_to_string(&mut input)?;

	let config: Config = toml::from_str(&input)?;

	let args = Args::parse();

	let tree = Tree::parse(
		config,
		args.tags
			.iter()
			.map(|tag| tag.to_string_lossy().into())
			.collect(),
	)?;

	tree.analyze()?;

	let stdout = io::stdout();
	let mut handle = stdout.lock();

	if args.link {
		let _results = tree.link()?;
	} else {
		write!(handle, "{}", tree)?;
	}

	Ok(())
}
