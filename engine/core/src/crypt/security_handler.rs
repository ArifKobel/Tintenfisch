use md5::{Digest, Md5};
use sha2::Sha256;

use crate::crypt::Permissions;
use crate::crypt::aes_cbc::{decrypt_blocks, decrypt_with_iv, encrypt_with_iv};
use crate::crypt::password::{
    LegacyParameters, legacy_file_key, legacy_user_from_owner, legacy_user_value, modern_hash, padded,
};
use crate::crypt::rc4::rc4;
use crate::error::{Error, Result};
use crate::syntax::{Dict, ObjId, Object, Stream};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CryptMethod {
    Identity,
    Rc4,
    AesV2,
    AesV3,
}

#[derive(Clone, Debug)]
pub struct SecurityHandler {
    key: Vec<u8>,
    strings: CryptMethod,
    streams: CryptMethod,
    encrypt_metadata: bool,
    dictionary: Option<ObjId>,
    permissions: i32,
    owner: bool,
}

impl SecurityHandler {
    pub fn unlock(
        encrypt: &Dict,
        dictionary: Option<ObjId>,
        first_id: &[u8],
        password: &str,
    ) -> Result<SecurityHandler> {
        if !encrypt.name("Filter").is_some_and(|filter| filter.is("Standard")) {
            return Err(Error::new("only the standard security handler is supported"));
        }
        let version = encrypt.int("V").unwrap_or(0);
        let revision = encrypt.int("R").unwrap_or(2);
        let bytes = |key: &str| encrypt.get(key).and_then(Object::as_string).unwrap_or_default();
        let encrypt_metadata = encrypt.get("EncryptMetadata").and_then(Object::as_bool).unwrap_or(true);
        let (strings, streams) = crypt_methods(encrypt, version);
        let permissions = encrypt.int("P").unwrap_or(-1) as i32;
        let (key, owner) = if revision >= 5 {
            modern_key(revision, password.as_bytes(), bytes("O"), bytes("U"), bytes("OE"), bytes("UE"))?
        } else {
            let key_length = match version {
                1 => 5,
                _ => (encrypt.int("Length").unwrap_or(40) as usize / 8).clamp(5, 16),
            };
            let parameters =
                LegacyParameters { revision, key_length, owner: bytes("O"), permissions, first_id, encrypt_metadata };
            legacy_key(&parameters, password.as_bytes(), bytes("U"))?
        };
        Ok(SecurityHandler { key, strings, streams, encrypt_metadata, dictionary, permissions, owner })
    }

    pub fn sealed(key: Vec<u8>, permissions: i32) -> SecurityHandler {
        let (strings, streams) = (CryptMethod::AesV3, CryptMethod::AesV3);
        SecurityHandler { key, strings, streams, encrypt_metadata: true, dictionary: None, permissions, owner: true }
    }

    pub fn has_owner_rights(&self) -> bool {
        self.owner
    }

    pub fn permissions(&self) -> Permissions {
        Permissions::from_bits(self.permissions)
    }

    pub fn is_dictionary(&self, id: ObjId) -> bool {
        self.dictionary == Some(id)
    }

    pub fn decrypt(&self, id: ObjId, object: Object) -> Object {
        self.transform(id, object, false)
    }

    pub fn encrypt(&self, id: ObjId, object: Object) -> Object {
        self.transform(id, object, true)
    }

    fn transform(&self, id: ObjId, object: Object, encrypting: bool) -> Object {
        match object {
            Object::String(bytes) => Object::String(self.apply(id, self.strings, &bytes, encrypting)),
            Object::Array(items) => {
                Object::Array(items.into_iter().map(|item| self.transform(id, item, encrypting)).collect())
            }
            Object::Dict(dict) => Object::Dict(self.transform_dict(id, dict, encrypting)),
            Object::Stream(stream) => {
                let skip = self.leaves_stream_alone(&stream);
                let dict = self.transform_dict(id, stream.dict, encrypting);
                let raw = if skip { stream.raw } else { self.apply(id, self.streams, &stream.raw, encrypting).into() };
                Object::Stream(Stream { dict, raw })
            }
            other => other,
        }
    }

    fn transform_dict(&self, id: ObjId, dict: Dict, encrypting: bool) -> Dict {
        dict.iter().map(|(key, value)| (key.clone(), self.transform(id, value.clone(), encrypting))).collect()
    }

    fn leaves_stream_alone(&self, stream: &Stream) -> bool {
        let dict = &stream.dict;
        let identity_filter = dict
            .get("Filter")
            .map(|filter| match filter {
                Object::Name(name) => vec![name.clone()],
                Object::Array(items) => items.iter().filter_map(Object::as_name).cloned().collect(),
                _ => Vec::new(),
            })
            .is_some_and(|names| names.iter().any(|name| name.is("Crypt")));
        dict.has_type("XRef") || (dict.has_type("Metadata") && !self.encrypt_metadata) || identity_filter
    }

    fn apply(&self, id: ObjId, method: CryptMethod, data: &[u8], encrypting: bool) -> Vec<u8> {
        let key = self.object_key(id, method);
        match method {
            CryptMethod::Identity => data.to_vec(),
            CryptMethod::Rc4 => rc4(&key, data),
            CryptMethod::AesV2 | CryptMethod::AesV3 if encrypting => {
                let iv: [u8; 16] = Sha256::digest([key.as_slice(), data].concat())[..16].try_into().unwrap_or([0; 16]);
                encrypt_with_iv(&key, iv, data).unwrap_or_default()
            }
            CryptMethod::AesV2 | CryptMethod::AesV3 => decrypt_with_iv(&key, data).unwrap_or_default(),
        }
    }

    fn object_key(&self, id: ObjId, method: CryptMethod) -> Vec<u8> {
        if method == CryptMethod::AesV3 {
            return self.key.clone();
        }
        let mut hasher = Md5::new();
        hasher.update(&self.key);
        hasher.update(&id.num.to_le_bytes()[..3]);
        hasher.update(id.generation.to_le_bytes());
        if method == CryptMethod::AesV2 {
            hasher.update(b"sAlT");
        }
        hasher.finalize()[..(self.key.len() + 5).min(16)].to_vec()
    }
}

fn crypt_methods(encrypt: &Dict, version: i64) -> (CryptMethod, CryptMethod) {
    if version < 4 {
        return (CryptMethod::Rc4, CryptMethod::Rc4);
    }
    let filters = encrypt.get("CF").and_then(Object::as_dict);
    let method = |key: &str| {
        let name = encrypt.name(key)?;
        if name.is("Identity") {
            return Some(CryptMethod::Identity);
        }
        let filter = filters?.get_name(name)?.as_dict()?;
        Some(match filter.name("CFM").map(|method| method.as_bytes()) {
            Some(b"AESV2") => CryptMethod::AesV2,
            Some(b"AESV3") => CryptMethod::AesV3,
            Some(b"None") => CryptMethod::Identity,
            _ => CryptMethod::Rc4,
        })
    };
    let identity = CryptMethod::Identity;
    (method("StrF").unwrap_or(identity), method("StmF").unwrap_or(identity))
}

fn legacy_key(parameters: &LegacyParameters, password: &[u8], user_value: &[u8]) -> Result<(Vec<u8>, bool)> {
    let compared = if parameters.revision == 2 { 32 } else { 16 };
    let matches = |key: &[u8]| {
        let expected = legacy_user_value(key, parameters.revision, parameters.first_id);
        user_value.len() >= compared && expected[..compared] == user_value[..compared]
    };
    let as_owner = legacy_file_key(&legacy_user_from_owner(password, parameters), parameters);
    if matches(&as_owner) {
        return Ok((as_owner, true));
    }
    let as_user = legacy_file_key(&padded(password), parameters);
    if matches(&as_user) {
        return Ok((as_user, false));
    }
    Err(Error::needs_password("this document needs a password"))
}

fn modern_key(
    revision: i64,
    password: &[u8],
    owner: &[u8],
    user: &[u8],
    owner_key: &[u8],
    user_key: &[u8],
) -> Result<(Vec<u8>, bool)> {
    if owner.len() < 48 || user.len() < 48 {
        return Err(Error::new("/O or /U too short for revision 5 or 6"));
    }
    let unwrap = |intermediate: [u8; 32], wrapped: &[u8]| {
        decrypt_blocks(&intermediate, [0; 16], wrapped.get(..32).unwrap_or_default())
    };
    if modern_hash(revision, password, &owner[32..40], &user[..48]) == owner[..32] {
        return Ok((unwrap(modern_hash(revision, password, &owner[40..48], &user[..48]), owner_key)?, true));
    }
    if modern_hash(revision, password, &user[32..40], &[]) == user[..32] {
        return Ok((unwrap(modern_hash(revision, password, &user[40..48], &[]), user_key)?, false));
    }
    Err(Error::needs_password("this document needs a password"))
}
