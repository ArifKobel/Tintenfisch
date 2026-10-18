use cms::content_info::ContentInfo;
use cms::encrypted_data::EncryptedData;
use der::asn1::OctetString;
use der::{Decode, Encode};
use hmac::{Hmac, KeyInit, Mac};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use x509_cert::Certificate;

use crate::error::{Error, Result};
use crate::signing::oids;
use crate::signing::password_decryption::{PasswordDecryption, wrong_password};
use crate::signing::pkcs12_kdf::{KeyPurpose, bmp_password, derive};
use crate::signing::pkcs12_structures::{CertBag, EncryptedKey, MacData, Pfx, SafeBag};

pub struct Pkcs12Archive {
    pub keys: Vec<Vec<u8>>,
    pub certificates: Vec<Certificate>,
}

impl Pkcs12Archive {
    pub fn open(bytes: &[u8], password: &str) -> Result<Pkcs12Archive> {
        let pfx = Pfx::from_der(bytes)?;
        if pfx.auth_safe.content_type != oids::DATA {
            return Err(Error::unsupported("certificate files protected by a public key are not supported"));
        }
        let safe = pfx.auth_safe.content.decode_as::<OctetString>()?;
        if let Some(mac) = &pfx.mac_data {
            check_mac(mac, safe.as_bytes(), password)?;
        }
        let decryption = PasswordDecryption::new(password);
        let mut archive = Pkcs12Archive { keys: Vec::new(), certificates: Vec::new() };
        for content in Vec::<ContentInfo>::from_der(safe.as_bytes())? {
            let bags = match content.content_type {
                oid if oid == oids::DATA => content.content.decode_as::<OctetString>()?.as_bytes().to_vec(),
                oid if oid == oids::ENCRYPTED_DATA => {
                    let encrypted = content.content.decode_as::<EncryptedData>()?;
                    let info = encrypted.enc_content_info;
                    let data =
                        info.encrypted_content.ok_or_else(|| Error::new("encrypted certificate data is empty"))?;
                    decryption.decrypt(&info.content_enc_alg, data.as_bytes())?
                }
                _ => continue,
            };
            for bag in Vec::<SafeBag>::from_der(&bags)? {
                match bag.bag_id {
                    oid if oid == oids::KEY_BAG => archive.keys.push(bag.bag_value.to_der()?),
                    oid if oid == oids::SHROUDED_KEY_BAG => {
                        let key = bag.bag_value.decode_as::<EncryptedKey>()?;
                        archive.keys.push(decryption.decrypt(&key.algorithm, key.data.as_bytes())?);
                    }
                    oid if oid == oids::CERT_BAG => {
                        let certificate = bag.bag_value.decode_as::<CertBag>()?;
                        if certificate.cert_id == oids::X509_CERTIFICATE {
                            archive.certificates.push(Certificate::from_der(certificate.cert_value.as_bytes())?);
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(archive)
    }
}

fn check_mac(mac: &MacData, content: &[u8], password: &str) -> Result<()> {
    let password = bmp_password(password);
    let salt = mac.mac_salt.as_bytes();
    let expected = mac.mac.digest.as_bytes();
    let computed = match mac.mac.algorithm.oid {
        oid if oid == oids::SHA1 => {
            let key = derive::<Sha1>(&password, salt, KeyPurpose::Mac, mac.iterations, Sha1::output_size());
            authenticate::<Hmac<Sha1>>(&key, content)?
        }
        oid if oid == oids::SHA256 => {
            let key = derive::<Sha256>(&password, salt, KeyPurpose::Mac, mac.iterations, Sha256::output_size());
            authenticate::<Hmac<Sha256>>(&key, content)?
        }
        oid => return Err(Error::unsupported(format!("unsupported certificate file checksum {oid}"))),
    };
    if computed != expected {
        return Err(wrong_password());
    }
    Ok(())
}

fn authenticate<M: Mac + KeyInit>(key: &[u8], content: &[u8]) -> Result<Vec<u8>> {
    let mut mac = <M as KeyInit>::new_from_slice(key).map_err(|_| Error::new("bad checksum key"))?;
    mac.update(content);
    Ok(mac.finalize().into_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checksum_catches_a_changed_byte() {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/certificates/test-signer.p12")).unwrap();
        let pfx = Pfx::from_der(&bytes).unwrap();
        let mac = pfx.mac_data.unwrap();
        let mut content = pfx.auth_safe.content.decode_as::<OctetString>().unwrap().as_bytes().to_vec();
        assert!(check_mac(&mac, &content, "tintenfisch-test").is_ok());
        let last = content.len() - 1;
        content[last] ^= 1;
        assert_eq!(
            check_mac(&mac, &content, "tintenfisch-test").unwrap_err().kind,
            crate::error::ErrorKind::NeedsPassword
        );
    }
}
