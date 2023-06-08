use std::result;
use std::{
    fmt::{self, Debug},
    io,
};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Compiler(String),
    Project(String),
    Build(String),
    Io(String),
    Mod(String),
    Diff(String),
    TOOL(String),
    NOTES(String),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(format!("{}", err))
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
        write!(f, "{}", self.description())
    }
}

impl Error {
    pub fn description(&self) -> String {
        match self {
            Error::Io(err) => err.to_string(),
            Error::Compiler(err) => err.to_string(),
            Error::Project(err) => err.to_string(),
            Error::Build(err) => err.to_string(),
            Error::Mod(err) => err.to_string(),
            Error::Diff(err) => format!("upatch-diff error, {}", err),
            Error::TOOL(err) => format!("upatch-tool error, {}", err),
            Error::NOTES(err) => format!("upatch-notes error, {}", err),
        }
    }

    pub fn code(&self) -> i32 {
        match self {
            Error::Io(_) => -1,
            Error::Compiler(_) => -2,
            Error::Project(_) => -3,
            Error::Build(_) => -4,
            Error::Mod(_) => -5,
            Error::Diff(_) => -6,
            Error::TOOL(_) => -7,
            Error::NOTES(_) => -8,
        }
    }
}
