use clap::Parser;

pub use clap;

/// Command-line arguments.
#[derive(Default, Parser)]
#[command(name = "park", about, version)]
#[command(long_about =
"park is a CLI tool that manages your dotfiles based on a configuration file
(more specifically, a TOML file).

By default, it shows a preview tree of how your target files would be
symlinked according to the configuration provided via stdin.

When in linking mode it tries to symlink target files according to the
preview tree."
)]
pub struct Park {
	/// Whether to symlink files.
	#[arg(long, short, help = "Try to link eligible targets")]
	pub link: bool,

	/// Runtime tags or file prefixes.
	#[arg(help = "List of additional tags or target names for filtering")]
	pub filters: Vec<String>,
}
