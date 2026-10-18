use der::Encode;
use pkcs8::{DecodePrivateKey, DecodePublicKey, PrivateKeyInfoRef};
use rsa::{RsaPrivateKey, RsaPublicKey};
use x509_cert::Certificate;
use x509_cert::name::Name;

use crate::error::{Error, Result};
use crate::signing::digest_algorithm::DigestAlgorithm;
use crate::signing::oids;
use crate::signing::pkcs12_archive::Pkcs12Archive;

pub struct SigningIdentity {
    key: RsaPrivateKey,
    certificate: Certificate,
    chain: Vec<Certificate>,
}

impl SigningIdentity {
    pub fn from_pkcs12(bytes: &[u8], password: &str) -> Result<SigningIdentity> {
        let archive = Pkcs12Archive::open(bytes, password)?;
        let key_info =
            archive.keys.first().ok_or_else(|| Error::unsupported("the certificate file holds no private key"))?;
        let key = RsaPrivateKey::from_pkcs8_der(key_info).map_err(|_| {
            let algorithm = <PrivateKeyInfoRef as der::Decode>::from_der(key_info).map(|info| info.algorithm.oid);
            match algorithm {
                Ok(oid) if oid != oids::RSA_ENCRYPTION => Error::unsupported("only RSA keys can sign so far"),
                _ => Error::new("the private key in the certificate file is unreadable"),
            }
        })?;
        let public = key.to_public_key();
        let (mine, chain): (Vec<Certificate>, Vec<Certificate>) = archive
            .certificates
            .into_iter()
            .partition(|certificate| public_key(certificate).is_some_and(|candidate| candidate == public));
        let certificate = mine
            .into_iter()
            .next()
            .ok_or_else(|| Error::unsupported("the certificate file has no certificate for its key"))?;
        Ok(SigningIdentity { key, certificate, chain })
    }

    pub fn certificate(&self) -> &Certificate {
        &self.certificate
    }

    pub fn chain(&self) -> &[Certificate] {
        &self.chain
    }

    pub fn signer_name(&self) -> String {
        common_name(self.certificate.tbs_certificate().subject())
    }

    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        let hash = DigestAlgorithm::Sha256.digest(&[message]);
        self.key
            .sign(DigestAlgorithm::Sha256.pkcs1v15(), &hash)
            .map_err(|error| Error::new(format!("signing failed: {error}")))
    }
}

pub fn public_key(certificate: &Certificate) -> Option<RsaPublicKey> {
    let info = certificate.tbs_certificate().subject_public_key_info().to_der().ok()?;
    RsaPublicKey::from_public_key_der(&info).ok()
}

pub fn common_name(name: &Name) -> String {
    match name.common_name() {
        Ok(Some(text)) => text.value().into_owned(),
        _ => name.to_string(),
    }
}
