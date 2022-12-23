use std::{
	error::Error as StdError,
	fmt::{Display, Formatter},
	io::{Error as IoError, ErrorKind as IoErrorKind},
	path::PathBuf,
};

use super::tree::Problems;

#[derive(Debug, PartialEq)]
pub enum Error {
	InternalError(PathBuf),
	IoError(IoErrorKind),
	BadFiles(Problems),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::InternalError(link_path) => {
				write!(f, "there's an error associated with {:?}", link_path)
			}
			Self::IoError(io_err) => IoError::new(*io_err, "unexpected IO error").fmt(f),
			Self::BadFiles(problems) => {
				let len = problems.len();

				writeln!(f, "found {} problematic target(s):", len)?;

				for (idx, (path, status)) in problems.iter().enumerate() {
					write!(f, "\t- {:?} at {:?}", status, path)?;

					if idx != len - 1 {
						writeln!(f)?;
					}
				}

				Ok(())
			}
		}
	}
}

impl StdError for Error {}
