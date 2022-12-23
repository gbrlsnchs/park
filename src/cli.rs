use clap::{ArgAction, Parser};

/// park is a CLI tool for managing dotfiles based on a TOML file.
///
/// See park(1) for more details about usage, and park(5) for how to use a
/// configuration file with it.
#[derive(Default, Parser)]
#[command(
	about,
	long_about,
	version,
	max_term_width = 80,
	disable_help_flag = true,
	disable_version_flag = true
)]
pub struct Park {
	/// Execute the linking step.
	///
	/// If any problems are detected during analysis, the linking step will be aborted and
	/// all problematic files will be listed.
	#[arg(long, short)]
	pub link: bool,

	/// Replace mismatched symlinks.
	///
	/// This allows bypassing the MISMATCH status by forcing the existing symlink to be
	/// replaced.
	#[arg(long, short)]
	pub replace: bool,

	/// Create parent directories when needed.
	///
	/// This will prevent links with status UNPARENTED to return an error during the linking
	/// step by creating all necessary directories that compose the symlink's path.
	#[arg(long, short)]
	pub create_dirs: bool,

	/// Show help usage.
	///
	/// Use -h to show the short help, or --help to show the long one (or even better,
	/// read the man pages).
	#[arg(long, short, action = ArgAction::Help)]
	pub help: Option<bool>,

	/// Show version.
	///
	/// The version format is 'park <version>'. Use it wisely.
	#[arg(long, short, action = ArgAction::Version)]
	pub version: Option<bool>,

	/// List of tags (appended with a plus sign) or target names (for filtering purposes).
	#[arg()]
	pub filters: Vec<String>,
}
