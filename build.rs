use std::{
	fs,
	io::{Error, Write},
	path::{Path, PathBuf},
	process::{Command, Stdio},
};

use crate::cli::Args;

use clap::CommandFactory;
use clap_complete::{self, Shell};
use clap_mangen::Man;

#[path = "src/cli.rs"]
mod cli;

fn main() -> Result<(), Error> {
	println!("cargo:rerun-if-changed=doc");

	let target_dir = PathBuf::from("target");

	let completions_dir = target_dir.join("completions");
	fs::create_dir_all(&completions_dir)?;

	let mut app = Args::command();
	let app_name = app.get_name().to_string();

	for shell in &[Shell::Bash, Shell::Zsh, Shell::Fish] {
		clap_complete::generate_to(*shell, &mut app, &app_name, &completions_dir)?;
	}

	if Command::new("scdoc").spawn().is_err() {
		eprintln!("scdoc not found in PATH, skipping generating manpage templates from doc/");

		return Ok(());
	}

	build_manpages(app, &target_dir)?;

	Ok(())
}

fn build_manpages(app: App, target_dir: &Path) -> Result<(), Error> {
	let doc_dir = target_dir.join("doc");
	fs::create_dir_all(&doc_dir)?;

	let man = Man::new(app);
	let mut buffer: Vec<u8> = Default::default();
	man.render_name_section(&mut buffer)?;
	man.render_synopsis_section(&mut buffer)?;
	man.render_description_section(&mut buffer)?;
	man.render_options_section(&mut buffer)?;

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

		let content = std::str::from_utf8(&output.stdout).unwrap();
		let content = content
			.replace("{{data}}", std::str::from_utf8(&buffer).unwrap())
			.replace("\n\n.P", "\n.P");

		fs::write(doc_dir.join(doc), content)?;
	}

	Ok(())
}
