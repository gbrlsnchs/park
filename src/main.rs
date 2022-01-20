use std::io::{self, Read};

use cli::Result as CliResult;

mod cli;
mod config;
mod tree;

fn main() -> CliResult {
	let mut input = String::new();

	let stdin = io::stdin();
	let mut handle = stdin.lock();
	handle.read_to_string(&mut input)?;

	let stdout = io::stdout();
	let handle = stdout.lock();

	cli::run(&input, handle)?;

	Ok(())
}
