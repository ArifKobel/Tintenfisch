use std::fmt;

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Name(Vec<u8>);

impl Name {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn is(&self, other: &str) -> bool {
        self.0 == other.as_bytes()
    }
}

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "/{}", String::from_utf8_lossy(&self.0))
    }
}
