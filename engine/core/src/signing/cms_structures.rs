use der::Sequence;
use der::asn1::OctetString;

#[derive(Clone, Debug, Sequence)]
pub struct SigningCertificateV2 {
    pub certs: Vec<EssCertIdV2>,
}

#[derive(Clone, Debug, Sequence)]
pub struct EssCertIdV2 {
    pub cert_hash: OctetString,
}
