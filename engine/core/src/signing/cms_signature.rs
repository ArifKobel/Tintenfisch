use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
use cms::content_info::{CmsVersion, ContentInfo};
use cms::signed_data::{EncapsulatedContentInfo, SignedData, SignerIdentifier, SignerInfo};
use der::asn1::{Any, ObjectIdentifier, OctetString, SetOfVec};
use der::{Encode, Tag};
use x509_cert::attr::Attribute;
use x509_cert::spki::AlgorithmIdentifierOwned;

use crate::error::Result;
use crate::signing::cms_structures::{EssCertIdV2, SigningCertificateV2};
use crate::signing::digest_algorithm::DigestAlgorithm;
use crate::signing::oids;
use crate::signing::signing_identity::SigningIdentity;

pub struct CmsSignature;

impl CmsSignature {
    pub fn create(identity: &SigningIdentity, digest: &[u8]) -> Result<Vec<u8>> {
        let certificate = identity.certificate();
        let certificate_hash = DigestAlgorithm::Sha256.digest(&[&certificate.to_der()?]);
        let signing_certificate =
            SigningCertificateV2 { certs: vec![EssCertIdV2 { cert_hash: OctetString::new(certificate_hash)? }] };
        let attributes = SetOfVec::try_from(vec![
            attribute(oids::CONTENT_TYPE, Any::encode_from(&oids::DATA)?)?,
            attribute(oids::MESSAGE_DIGEST, Any::encode_from(&OctetString::new(digest)?)?)?,
            attribute(oids::SIGNING_CERTIFICATE_V2, Any::encode_from(&signing_certificate)?)?,
        ])?;
        let signature = identity.sign(&attributes.to_der()?)?;
        let tbs = certificate.tbs_certificate();
        let signer = SignerInfo {
            version: CmsVersion::V1,
            sid: SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
                issuer: tbs.issuer().clone(),
                serial_number: tbs.serial_number().clone(),
            }),
            digest_alg: algorithm(oids::SHA256, None),
            signed_attrs: Some(attributes),
            signature_algorithm: algorithm(oids::RSA_ENCRYPTION, Some(Any::new(Tag::Null, Vec::new())?)),
            signature: OctetString::new(signature)?,
            unsigned_attrs: None,
        };
        let certificates = std::iter::once(certificate)
            .chain(identity.chain())
            .cloned()
            .map(CertificateChoices::Certificate)
            .collect::<Vec<_>>();
        let signed = SignedData {
            version: CmsVersion::V1,
            digest_algorithms: SetOfVec::try_from(vec![algorithm(oids::SHA256, None)])?,
            encap_content_info: EncapsulatedContentInfo { econtent_type: oids::DATA, econtent: None },
            certificates: Some(certificates.try_into()?),
            crls: None,
            signer_infos: vec![signer].try_into()?,
        };
        Ok(ContentInfo { content_type: oids::SIGNED_DATA, content: Any::encode_from(&signed)? }.to_der()?)
    }
}

fn attribute(oid: ObjectIdentifier, value: Any) -> Result<Attribute> {
    Ok(Attribute { oid, values: SetOfVec::try_from(vec![value])? })
}

fn algorithm(oid: ObjectIdentifier, parameters: Option<Any>) -> AlgorithmIdentifierOwned {
    AlgorithmIdentifierOwned { oid, parameters }
}
