use clap::Parser;

/// Command-line arguments.
#[derive(Default, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
	/// Whether to symlink files.
	#[clap(long, short, help = "Try to link eligible targets")]
	pub link: bool,

	/// Runtime tags or file prefixes.
	#[clap(value_parser, help = "List of additional tags")]
	pub filters: Vec<String>,
}
