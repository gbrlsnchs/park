use clap::Parser;

pub use clap;

/// Command-line arguments.
#[derive(Default, Parser)]
#[command(about, version)]
pub struct Park {
	/// Whether to symlink files.
	#[arg(long, short, help = "Try to link eligible targets")]
	pub link: bool,

	/// Runtime tags or file prefixes.
	#[arg(help = "List of additional tags or target names for filtering")]
	pub filters: Vec<String>,
}
