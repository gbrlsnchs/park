use clap::Parser;

/// park is a CLI tool for managing dotfiles based on a TOML file.
///
/// See park(1) for more details about usage, and park(5) for how to use a
/// configuration file with it.
#[derive(Default, Parser)]
#[command(about, long_about, version, max_term_width = 80)]
pub struct Park {
	/// Whether to symlink files.
	#[arg(long, short, help = "Try to link eligible targets")]
	pub link: bool,

	/// Runtime tags or file prefixes.
	#[arg(help = "List of additional tags or target names for filtering")]
	pub filters: Vec<String>,
}
