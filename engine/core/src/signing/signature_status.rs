#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignatureStatus {
    Valid,
    Broken(String),
    Unsupported(String),
}
