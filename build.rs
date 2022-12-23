use std::{
	fs,
	io::{Error, Write},
	path::{Path, PathBuf},
	process::{Command, Stdio},
};

use crate::cli::Park;

use clap::CommandFactory;
use clap_complete::{self, Shell};

#[path = "src/cli.rs"]
mod cli;

fn main() -> Result<(), Error> {
	println!("cargo:rerun-if-changed=doc");

	let target_dir = PathBuf::from("target");

	let completions_dir = target_dir.join("completions");
	fs::create_dir_all(&completions_dir)?;

	let mut app = Park::command();
	let app_name = app.get_name().to_string();

	for shell in &[Shell::Bash, Shell::Zsh, Shell::Fish] {
		clap_complete::generate_to(*shell, &mut app, &app_name, &completions_dir)?;
	}

	if Command::new("scdoc").spawn().is_err() {
		eprintln!("scdoc not found in PATH, skipping generating manpage templates from doc/");

		return Ok(());
	}

	build_manpages(&target_dir)?;

	Ok(())
}

fn build_manpages(target_dir: &Path) -> Result<(), Error> {
	let doc_dir = target_dir.join("doc");
	fs::create_dir_all(&doc_dir)?;

	for doc in fs::read_dir("doc")? {
		let doc = doc?;

		let cmd = Command::new("scdoc")
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn();

		let mut cmd = cmd?;

		if let Some(mut stdin) = cmd.stdin.take() {
			let doc = fs::read(doc.path())?;
			stdin.write_all(&doc)?;
		}

		let output = cmd.wait_with_output()?;
		let doc = PathBuf::from(doc.file_name());
		let doc = doc.file_stem().unwrap();

		fs::write(doc_dir.join(doc), output.stdout)?;
	}

	Ok(())
}
