use std::{env, io::Error};

use crate::cli::Args;

use clap::IntoApp;
use clap_complete::{self, Shell};

#[path = "src/cli.rs"]
mod cli;

fn main() -> Result<(), Error> {
	let out_dir = match env::var_os("OUT_DIR") {
		None => return Ok(()),
		Some(outdir) => outdir,
	};

	let mut app = Args::into_app();
	let app_name = app.get_name().to_string();

	for shell in &[Shell::Bash, Shell::Zsh] {
		clap_complete::generate_to(*shell, &mut app, &app_name, &out_dir)?;
	}

	Ok(())
}
