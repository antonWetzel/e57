use std::fmt::Result as FmtResult;
use std::fmt::{Display, Formatter};

/// Possible errors that can occur while working with E57 files.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
	IO(std::io::Error),
	/// The E57 you are reading or creating is invalid and does not confirm with the E57 format specification.
	Invalid(String),
	Unimplemented(String),

	Parse(std::num::ParseIntError),
	Utf8(std::string::FromUtf8Error),
	XML(roxmltree::Error),
}

pub static INTERNAL_ERROR: &str = "internal error";

impl Error {
	pub fn throw<T>(self) -> Result<T, Error> {
		Err(self)
	}
}

impl From<std::io::Error> for Error {
	fn from(value: std::io::Error) -> Self {
		Self::IO(value)
	}
}
impl From<std::num::ParseIntError> for Error {
	fn from(value: std::num::ParseIntError) -> Self {
		Self::Parse(value)
	}
}
impl From<std::string::FromUtf8Error> for Error {
	fn from(value: std::string::FromUtf8Error) -> Self {
		Self::Utf8(value)
	}
}
impl From<roxmltree::Error> for Error {
	fn from(value: roxmltree::Error) -> Self {
		Self::XML(value)
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		match self {
			Error::Invalid(desc) => write!(f, "Invalid E57 content: {desc}"),
			Error::Unimplemented(desc) => write!(f, "Invalid E57 content: {desc}"),
			Error::IO(err) => write!(f, "{}", err),
			Error::Parse(err) => write!(f, "{}", err),
			Error::Utf8(err) => write!(f, "{}", err),
			Error::XML(err) => write!(f, "{}", err),
		}
	}
}
