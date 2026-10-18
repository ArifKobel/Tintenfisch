use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Malformed,
    NeedsPassword,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub kind: ErrorKind,
    pub offset: Option<usize>,
    pub message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Malformed, offset: None, message: message.into() }
    }

    pub fn at(offset: usize, message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Malformed, offset: Some(offset), message: message.into() }
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::Unsupported, offset: None, message: message.into() }
    }

    pub fn needs_password(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::NeedsPassword, offset: None, message: message.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.offset {
            Some(offset) => write!(f, "{} at byte {}", self.message, offset),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

impl From<der::Error> for Error {
    fn from(error: der::Error) -> Self {
        Error::new(format!("malformed signature data: {error}"))
    }
}
