use std::io::{self, Read};

use clap::Parser;

use cli::{Args, Result as CliResult};

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

	let args = Args::parse();

	cli::run(&input, handle, args)?;

	Ok(())
}
