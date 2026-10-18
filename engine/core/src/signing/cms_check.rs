use cms::cert::CertificateChoices;
use cms::content_info::ContentInfo;
use cms::signed_data::{SignedData, SignerIdentifier, SignerInfo};
use der::asn1::OctetString;
use der::{Decode, Encode, SliceReader};
use x509_cert::Certificate;
use x509_cert::ext::pkix::SubjectKeyIdentifier;

use crate::signing::digest_algorithm::DigestAlgorithm;
use crate::signing::oids;
use crate::signing::signature_status::SignatureStatus;
use crate::signing::signing_identity::{common_name, public_key};

pub struct CmsCheck {
    pub status: SignatureStatus,
    pub signer: Option<String>,
}

impl CmsCheck {
    pub fn run(cms: &[u8], signed: &[&[u8]]) -> CmsCheck {
        let signed_data = SliceReader::new(cms)
            .and_then(|mut reader| ContentInfo::decode(&mut reader))
            .ok()
            .filter(|info| info.content_type == oids::SIGNED_DATA)
            .and_then(|info| info.content.decode_as::<SignedData>().ok());
        let Some(signed_data) = signed_data else {
            return CmsCheck {
                status: SignatureStatus::Broken("the signature is not readable".to_string()),
                signer: None,
            };
        };
        let Some(signer_info) = signed_data.signer_infos.0.iter().next() else {
            return CmsCheck {
                status: SignatureStatus::Broken("the signature names no signer".to_string()),
                signer: None,
            };
        };
        let certificate = signer_certificate(&signed_data, &signer_info.sid);
        let signer = certificate.map(|certificate| common_name(certificate.tbs_certificate().subject()));
        let status = match certificate {
            Some(certificate) => verdict(signer_info, certificate, signed),
            None => SignatureStatus::Broken("the signer's certificate is missing".to_string()),
        };
        CmsCheck { status, signer }
    }
}

fn verdict(signer_info: &SignerInfo, certificate: &Certificate, signed: &[&[u8]]) -> SignatureStatus {
    let Some(digest_algorithm) = DigestAlgorithm::from_oid(signer_info.digest_alg.oid) else {
        return SignatureStatus::Unsupported(format!("the hash {} cannot be checked", signer_info.digest_alg.oid));
    };
    let signature_oid = signer_info.signature_algorithm.oid;
    let hash = if signature_oid == oids::RSA_ENCRYPTION {
        Some(digest_algorithm)
    } else {
        DigestAlgorithm::of_rsa_signature(signature_oid)
    };
    let Some(hash) = hash else {
        return SignatureStatus::Unsupported(format!("signatures made with {signature_oid} cannot be checked yet"));
    };
    let digest = digest_algorithm.digest(signed);
    let message = match &signer_info.signed_attrs {
        Some(attributes) => {
            let claimed = attributes
                .iter()
                .find(|attribute| attribute.oid == oids::MESSAGE_DIGEST)
                .and_then(|attribute| attribute.values.iter().next())
                .and_then(|value| value.decode_as::<OctetString>().ok());
            if claimed.as_ref().map(OctetString::as_bytes) != Some(digest.as_slice()) {
                return SignatureStatus::Broken("the signed bytes have changed".to_string());
            }
            match attributes.to_der() {
                Ok(encoded) => hash.digest(&[&encoded]),
                Err(_) => return SignatureStatus::Broken("the signed attributes are not readable".to_string()),
            }
        }
        None => digest,
    };
    let Some(key) = public_key(certificate) else {
        return SignatureStatus::Unsupported("only RSA signatures can be checked so far".to_string());
    };
    match key.verify(hash.pkcs1v15(), &message, signer_info.signature.as_bytes()) {
        Ok(()) => SignatureStatus::Valid,
        Err(_) => SignatureStatus::Broken("the signature does not match the signer's certificate".to_string()),
    }
}

fn signer_certificate<'d>(signed_data: &'d SignedData, sid: &SignerIdentifier) -> Option<&'d Certificate> {
    signed_data.certificates.as_ref()?.0.iter().find_map(|choice| match choice {
        CertificateChoices::Certificate(certificate) => {
            let tbs = certificate.tbs_certificate();
            let matches = match sid {
                SignerIdentifier::IssuerAndSerialNumber(issuer) => {
                    tbs.issuer() == &issuer.issuer && tbs.serial_number() == &issuer.serial_number
                }
                SignerIdentifier::SubjectKeyIdentifier(wanted) => {
                    tbs.get_extension::<SubjectKeyIdentifier>().ok().flatten().is_some_and(|(_, own)| &own == wanted)
                }
            };
            matches.then_some(certificate)
        }
        _ => None,
    })
}
