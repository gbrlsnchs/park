use cli::Result as CliResult;

mod cli;
mod config;
mod tree;

fn main() -> CliResult {
	cli::run()?;

	Ok(())
}
