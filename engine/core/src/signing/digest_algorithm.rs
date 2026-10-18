use der::asn1::ObjectIdentifier;
use rsa::Pkcs1v15Sign;
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha384, Sha512};

use crate::signing::oids;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DigestAlgorithm {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl DigestAlgorithm {
    pub fn from_oid(oid: ObjectIdentifier) -> Option<DigestAlgorithm> {
        [
            (oids::SHA1, DigestAlgorithm::Sha1),
            (oids::SHA256, DigestAlgorithm::Sha256),
            (oids::SHA384, DigestAlgorithm::Sha384),
            (oids::SHA512, DigestAlgorithm::Sha512),
        ]
        .into_iter()
        .find(|(known, _)| *known == oid)
        .map(|(_, algorithm)| algorithm)
    }

    pub fn of_rsa_signature(oid: ObjectIdentifier) -> Option<DigestAlgorithm> {
        [
            (oids::SHA1_WITH_RSA, DigestAlgorithm::Sha1),
            (oids::SHA256_WITH_RSA, DigestAlgorithm::Sha256),
            (oids::SHA384_WITH_RSA, DigestAlgorithm::Sha384),
            (oids::SHA512_WITH_RSA, DigestAlgorithm::Sha512),
        ]
        .into_iter()
        .find(|(known, _)| *known == oid)
        .map(|(_, algorithm)| algorithm)
    }

    pub fn digest(self, parts: &[&[u8]]) -> Vec<u8> {
        fn run<D: Digest>(parts: &[&[u8]]) -> Vec<u8> {
            parts.iter().fold(D::new(), |hash, part| hash.chain_update(part)).finalize().to_vec()
        }
        match self {
            DigestAlgorithm::Sha1 => run::<Sha1>(parts),
            DigestAlgorithm::Sha256 => run::<Sha256>(parts),
            DigestAlgorithm::Sha384 => run::<Sha384>(parts),
            DigestAlgorithm::Sha512 => run::<Sha512>(parts),
        }
    }

    pub fn pkcs1v15(self) -> Pkcs1v15Sign {
        match self {
            DigestAlgorithm::Sha1 => Pkcs1v15Sign::new::<Sha1>(),
            DigestAlgorithm::Sha256 => Pkcs1v15Sign::new::<Sha256>(),
            DigestAlgorithm::Sha384 => Pkcs1v15Sign::new::<Sha384>(),
            DigestAlgorithm::Sha512 => Pkcs1v15Sign::new::<Sha512>(),
        }
    }
}
