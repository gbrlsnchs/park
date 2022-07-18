use std::io::{self, Read};

use clap::Parser;

use cli::Args;
use run::Result as RunResult;

mod cli;
mod config;
mod run;
mod tree;

// TODO: Test CLI interactions.
fn main() -> RunResult {
	let args = Args::parse();

	let mut input = String::new();

	let stdin = io::stdin();
	let mut handle = stdin.lock();
	handle.read_to_string(&mut input)?;

	let stdout = io::stdout();
	let handle = stdout.lock();

	run::run(&input, handle, args)?;

	Ok(())
}
