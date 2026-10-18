use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockModeDecrypt, InnerIvInit, KeyIvInit};
use der::{Decode, Encode};
use des::TdesEde3;
use pkcs8::pkcs5::EncryptionScheme;
use rc2::Rc2;
use sha1::Sha1;
use x509_cert::spki::AlgorithmIdentifierOwned;

use crate::error::{Error, Result};
use crate::signing::oids;
use crate::signing::pkcs12_kdf::{KeyPurpose, bmp_password, derive};
use crate::signing::pkcs12_structures::Pkcs12PbeParams;

pub struct PasswordDecryption<'p> {
    password: &'p str,
}

impl<'p> PasswordDecryption<'p> {
    pub fn new(password: &'p str) -> Self {
        Self { password }
    }

    pub fn decrypt(&self, algorithm: &AlgorithmIdentifierOwned, data: &[u8]) -> Result<Vec<u8>> {
        if algorithm.oid == oids::PBES2 {
            let scheme = EncryptionScheme::try_from(algorithm.to_der()?.as_slice())
                .map_err(|error| Error::unsupported(format!("unsupported certificate file encryption: {error}")))?;
            return scheme.decrypt(self.password.as_bytes(), data).map_err(|_| wrong_password());
        }
        let params = algorithm
            .parameters
            .as_ref()
            .ok_or_else(|| Error::new("the certificate file encryption has no parameters"))?;
        let params = Pkcs12PbeParams::from_der(&params.to_der()?)?;
        let (key_length, effective_bits) = match algorithm.oid {
            oid if oid == oids::PBE_SHA1_3DES => (24, None),
            oid if oid == oids::PBE_SHA1_RC2_128 => (16, Some(128)),
            oid if oid == oids::PBE_SHA1_RC2_40 => (5, Some(40)),
            oid => return Err(Error::unsupported(format!("unsupported certificate file encryption {oid}"))),
        };
        let password = bmp_password(self.password);
        let salt = params.salt.as_bytes();
        let key = derive::<Sha1>(&password, salt, KeyPurpose::Key, params.iterations, key_length);
        let iv = derive::<Sha1>(&password, salt, KeyPurpose::Iv, params.iterations, 8);
        let mut buffer = data.to_vec();
        let plain = match effective_bits {
            None => cbc::Decryptor::<TdesEde3>::new_from_slices(&key, &iv)
                .map_err(|_| Error::new("bad triple DES key"))?
                .decrypt_padded::<Pkcs7>(&mut buffer)
                .map_err(|_| wrong_password())?
                .to_vec(),
            Some(bits) => cbc::Decryptor::<Rc2>::inner_iv_slice_init(Rc2::new_with_eff_key_len(&key, bits), &iv)
                .map_err(|_| Error::new("bad RC2 initialisation vector"))?
                .decrypt_padded::<Pkcs7>(&mut buffer)
                .map_err(|_| wrong_password())?
                .to_vec(),
        };
        Ok(plain)
    }
}

pub fn wrong_password() -> Error {
    Error::needs_password("the password does not open the certificate file")
}
