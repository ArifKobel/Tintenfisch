use crate::signing::SignatureStatus;
use crate::syntax::ObjId;

#[derive(Clone, Debug, PartialEq)]
pub struct SignatureReport {
    pub field: ObjId,
    pub field_name: String,
    pub signer: Option<String>,
    pub signed_at: String,
    pub reason: String,
    pub status: SignatureStatus,
    pub changed_after: bool,
}
