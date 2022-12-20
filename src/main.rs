use std::{
	env,
	io::{self, Read},
};

use park_cli::clap::Parser;

use anyhow::Result;
use park_cli::Park;
use run::Env;

mod config;
mod parser;
mod printer;
mod run;

// TODO: Test CLI interactions.
fn main() -> Result<()> {
	let args = Park::parse();

	let mut input = String::new();

	let stdin = io::stdin();
	let mut handle = stdin.lock();
	handle.read_to_string(&mut input)?;

	let stdout = io::stdout();
	let handle = stdout.lock();

	run::run(
		Env {
			colored: env::var_os("NO_COLOR").is_none(),
			home: env::var_os("HOME"),
		},
		&input,
		handle,
		args,
	)?;

	Ok(())
}
