use std::{
	fs,
	io::{Error, Write},
	path::PathBuf,
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

	let man = Man::new(app);
	let mut buffer: Vec<u8> = Default::default();
	man.render_title(&mut buffer)?;
	man.render_name_section(&mut buffer)?;
	man.render_synopsis_section(&mut buffer)?;
	man.render_description_section(&mut buffer)?;
	man.render_options_section(&mut buffer)?;

	let footer: String = r".SH SEE ALSO
.P
\fIpark\fR(5)
.P
.SH AUTHORS
.P
Developed and maintained by Gabriel Sanches <gabriel@gsr.\&dev>.\&
.P
Source code is located at <https://git.\&sr.\&ht/~gbrlsnchs/park>.\&"
		.into();
	buffer.append(&mut footer.into_bytes());

	let doc_dir = target_dir.join("doc");
	fs::create_dir_all(&doc_dir)?;

	let manpage = PathBuf::from(format!("{}.1", app_name));
	fs::write(PathBuf::from(&doc_dir).join(manpage), buffer)?;

	for doc in fs::read_dir("doc")? {
		let doc = doc?;

		let cmd = Command::new("scdoc")
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn();

		if cmd.is_err() {
			eprintln!("scdoc not found in PATH, skipping generating manpage templates from doc/");
			break;
		}

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
