use std::{
	fmt::{Display, Formatter},
	io::{Error as IoError, ErrorKind as IoErrorKind},
	path::PathBuf,
};

#[derive(Debug, PartialEq)]
pub enum Error {
	InternalError(PathBuf),
	IoError(IoErrorKind),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::InternalError(link_path) => {
				write!(f, "there's an error associated with {:?}", link_path)
			}
			Self::IoError(io_err) => IoError::new(*io_err, "unexpected IO error").fmt(f),
		}
	}
}
