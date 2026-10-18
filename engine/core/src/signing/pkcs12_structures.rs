use cms::content_info::ContentInfo;
use der::Sequence;
use der::asn1::{Any, ObjectIdentifier, OctetString, SetOfVec};
use x509_cert::attr::Attribute;
use x509_cert::spki::AlgorithmIdentifierOwned;

#[derive(Clone, Debug, Sequence)]
pub struct Pfx {
    pub version: u8,
    pub auth_safe: ContentInfo,
    pub mac_data: Option<MacData>,
}

#[derive(Clone, Debug, Sequence)]
pub struct MacData {
    pub mac: DigestInfo,
    pub mac_salt: OctetString,
    #[asn1(default = "one_iteration")]
    pub iterations: u32,
}

#[derive(Clone, Debug, Sequence)]
pub struct DigestInfo {
    pub algorithm: AlgorithmIdentifierOwned,
    pub digest: OctetString,
}

#[derive(Clone, Debug, Sequence)]
pub struct SafeBag {
    pub bag_id: ObjectIdentifier,
    #[asn1(context_specific = "0", tag_mode = "EXPLICIT")]
    pub bag_value: Any,
    pub bag_attributes: Option<SetOfVec<Attribute>>,
}

#[derive(Clone, Debug, Sequence)]
pub struct CertBag {
    pub cert_id: ObjectIdentifier,
    #[asn1(context_specific = "0", tag_mode = "EXPLICIT")]
    pub cert_value: OctetString,
}

#[derive(Clone, Debug, Sequence)]
pub struct EncryptedKey {
    pub algorithm: AlgorithmIdentifierOwned,
    pub data: OctetString,
}

#[derive(Clone, Debug, Sequence)]
pub struct Pkcs12PbeParams {
    pub salt: OctetString,
    pub iterations: u32,
}

fn one_iteration() -> u32 {
    1
}
