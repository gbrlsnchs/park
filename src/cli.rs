use clap::Parser;

/// Command-line arguments.
#[derive(Default, Parser)]
#[command(about, version)]
pub struct Args {
	/// Whether to symlink files.
	#[arg(long, short, help = "Try to link eligible targets")]
	pub link: bool,

	/// Runtime tags or file prefixes.
	#[arg(help = "List of additional tags")]
	pub filters: Vec<String>,
}
