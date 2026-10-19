use crate::crypt::aes_cbc::encrypt_blocks;
use crate::crypt::password::modern_hash;
use crate::crypt::{Permissions, SecurityHandler};
use crate::error::{Error, Result};
use crate::syntax::{Dict, Name, Object};

const REVISION: i64 = 6;

#[derive(Clone, Debug, PartialEq)]
pub struct Protection {
    pub user_password: String,
    pub owner_password: String,
    pub permissions: Permissions,
}

impl Protection {
    pub const ENTROPY: usize = 68;

    pub fn seal(&self, entropy: &[u8]) -> Result<(Dict, SecurityHandler)> {
        let entropy: &[u8; Protection::ENTROPY] =
            entropy.try_into().map_err(|_| Error::new("sealing needs 68 random bytes"))?;
        let key = &entropy[..32];
        let (user_salts, owner_salts, check) = (&entropy[32..48], &entropy[48..64], &entropy[64..68]);
        let owner = if self.owner_password.is_empty() { &self.user_password } else { &self.owner_password };
        let user = self.user_password.as_bytes();
        let mut user_entry = modern_hash(REVISION, user, &user_salts[..8], &[]).to_vec();
        user_entry.extend_from_slice(user_salts);
        let user_key = encrypt_blocks(&modern_hash(REVISION, user, &user_salts[8..], &[]), [0; 16], key)?;
        let owner_bytes = owner.as_bytes();
        let mut owner_entry = modern_hash(REVISION, owner_bytes, &owner_salts[..8], &user_entry).to_vec();
        owner_entry.extend_from_slice(owner_salts);
        let owner_key =
            encrypt_blocks(&modern_hash(REVISION, owner_bytes, &owner_salts[8..], &user_entry), [0; 16], key)?;
        let bits = self.permissions.bits();
        let mut perms = bits.to_le_bytes().to_vec();
        perms.extend_from_slice(&[0xFF; 4]);
        perms.extend_from_slice(b"Tadb");
        perms.extend_from_slice(check);
        let perms = encrypt_blocks(key, [0; 16], &perms)?;
        let name = |value: &str| Object::Name(Name::from(value));
        let filter: Dict = [("AuthEvent", name("DocOpen")), ("CFM", name("AESV3")), ("Length", Object::Int(32))]
            .into_iter()
            .map(|(key, value)| (Name::from(key), value))
            .collect();
        let dictionary: Dict = [
            ("Filter", name("Standard")),
            ("V", Object::Int(5)),
            ("R", Object::Int(REVISION)),
            ("Length", Object::Int(256)),
            ("CF", Object::Dict([(Name::from("StdCF"), Object::Dict(filter))].into_iter().collect())),
            ("StmF", name("StdCF")),
            ("StrF", name("StdCF")),
            ("O", Object::String(owner_entry)),
            ("U", Object::String(user_entry)),
            ("OE", Object::String(owner_key)),
            ("UE", Object::String(user_key)),
            ("P", Object::Int(bits as i64)),
            ("Perms", Object::String(perms)),
            ("EncryptMetadata", Object::Bool(true)),
        ]
        .into_iter()
        .map(|(key, value)| (Name::from(key), value))
        .collect();
        Ok((dictionary, SecurityHandler::sealed(key.to_vec(), bits)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypt::aes_cbc::decrypt_blocks;

    #[test]
    fn perms_carry_the_permissions_under_the_file_key_as_algorithm_10_lays_them_out() {
        let entropy: Vec<u8> = (0..Protection::ENTROPY as u8).collect();
        let permissions = Permissions { copy: false, ..Permissions::ALL };
        let protection = Protection { user_password: "user".into(), owner_password: "owner".into(), permissions };
        let (dictionary, _) = protection.seal(&entropy).unwrap();
        let sealed = dictionary.get("Perms").and_then(Object::as_string).unwrap();
        let perms = decrypt_blocks(&entropy[..32], [0; 16], sealed).unwrap();
        assert_eq!(&perms[..4], &permissions.bits().to_le_bytes());
        assert_eq!(&perms[4..12], b"\xFF\xFF\xFF\xFFTadb");
        assert_eq!(&perms[12..], &entropy[64..68]);
    }
}
