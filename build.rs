use std::{fs, io::Error};

use crate::cli::Args;

use clap::CommandFactory;
use clap_complete::{self, Shell};

#[path = "src/cli.rs"]
mod cli;

fn main() -> Result<(), Error> {
	let completion_dir = "target/completions";

	fs::create_dir_all(completion_dir)?;

	let mut app = Args::command();
	let app_name = app.get_name().to_string();

	for shell in &[Shell::Bash, Shell::Zsh, Shell::Fish] {
		clap_complete::generate_to(*shell, &mut app, &app_name, completion_dir)?;
	}

	Ok(())
}
