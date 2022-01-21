use std::io::{self, Read};

use clap::Parser;

use cli::Args;
use command::Result as CommandResult;

mod cli;
mod command;
mod config;
mod tree;

fn main() -> CommandResult {
	let mut input = String::new();

	let stdin = io::stdin();
	let mut handle = stdin.lock();
	handle.read_to_string(&mut input)?;

	let stdout = io::stdout();
	let handle = stdout.lock();

	let args = Args::parse();

	command::run(&input, handle, args)?;

	Ok(())
}
