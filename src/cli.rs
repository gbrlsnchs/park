use std::ffi::OsString;

use clap::Parser;

/// Command-line arguments.
#[derive(Default, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
	/// Whether to symlink files.
	#[clap(long, short, help = "Try to link eligible targets")]
	pub link: bool,

	/// Runtime tags.
	#[clap(parse(from_os_str), help = "List of additional tags")]
	pub tags: Vec<OsString>,
}
