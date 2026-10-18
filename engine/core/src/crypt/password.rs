use md5::{Digest, Md5};
use sha2::{Sha256, Sha384, Sha512};

use crate::crypt::aes_cbc::encrypt_blocks;
use crate::crypt::rc4::rc4;

const PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08, 0x2E, 0x2E, 0x00,
    0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

pub struct LegacyParameters<'a> {
    pub revision: i64,
    pub key_length: usize,
    pub owner: &'a [u8],
    pub permissions: i32,
    pub first_id: &'a [u8],
    pub encrypt_metadata: bool,
}

pub fn padded(password: &[u8]) -> [u8; 32] {
    let mut padded = PADDING;
    let length = password.len().min(32);
    padded[..length].copy_from_slice(&password[..length]);
    padded[length..].copy_from_slice(&PADDING[..32 - length]);
    padded
}

pub fn legacy_file_key(padded_password: &[u8; 32], parameters: &LegacyParameters) -> Vec<u8> {
    let mut hasher = Md5::new();
    hasher.update(padded_password);
    hasher.update(parameters.owner);
    hasher.update(parameters.permissions.to_le_bytes());
    hasher.update(parameters.first_id);
    if parameters.revision >= 4 && !parameters.encrypt_metadata {
        hasher.update([0xFF; 4]);
    }
    let mut key = hasher.finalize().to_vec();
    let length = parameters.key_length.min(16);
    if parameters.revision >= 3 {
        for _ in 0..50 {
            key = Md5::digest(&key[..length]).to_vec();
        }
    }
    key.truncate(length);
    key
}

fn xor_key(key: &[u8], round: u8) -> Vec<u8> {
    key.iter().map(|byte| byte ^ round).collect()
}

pub fn legacy_user_value(key: &[u8], revision: i64, first_id: &[u8]) -> Vec<u8> {
    if revision == 2 {
        return rc4(key, &PADDING);
    }
    let mut hasher = Md5::new();
    hasher.update(PADDING);
    hasher.update(first_id);
    let mut value = rc4(key, &hasher.finalize());
    for round in 1..=19u8 {
        value = rc4(&xor_key(key, round), &value);
    }
    value
}

pub fn legacy_user_from_owner(owner_password: &[u8], parameters: &LegacyParameters) -> [u8; 32] {
    let length = parameters.key_length.min(16);
    let mut key = Md5::digest(padded(owner_password)).to_vec();
    if parameters.revision >= 3 {
        for _ in 0..50 {
            key = Md5::digest(&key).to_vec();
        }
    }
    key.truncate(length);
    let mut value = parameters.owner[..parameters.owner.len().min(32)].to_vec();
    if parameters.revision == 2 {
        value = rc4(&key, &value);
    } else {
        for round in (0..=19u8).rev() {
            value = rc4(&xor_key(&key, round), &value);
        }
    }
    let mut user = [0u8; 32];
    user[..value.len()].copy_from_slice(&value);
    user
}

pub fn modern_hash(revision: i64, password: &[u8], salt: &[u8], user_data: &[u8]) -> [u8; 32] {
    let password = &password[..password.len().min(127)];
    let mut hasher = Sha256::new();
    hasher.update(password);
    hasher.update(salt);
    hasher.update(user_data);
    let mut k = hasher.finalize().to_vec();
    if revision >= 6 {
        let mut round = 0usize;
        loop {
            let mut sequence = Vec::with_capacity(64 * (password.len() + k.len() + user_data.len()));
            for _ in 0..64 {
                sequence.extend_from_slice(password);
                sequence.extend_from_slice(&k);
                sequence.extend_from_slice(user_data);
            }
            let iv: [u8; 16] = k[16..32].try_into().unwrap_or([0; 16]);
            let encrypted = encrypt_blocks(&k[..16], iv, &sequence).unwrap_or_default();
            let selector: u32 = encrypted[..16].iter().map(|&byte| byte as u32).sum::<u32>() % 3;
            k = match selector {
                0 => Sha256::digest(&encrypted).to_vec(),
                1 => Sha384::digest(&encrypted).to_vec(),
                _ => Sha512::digest(&encrypted).to_vec(),
            };
            round += 1;
            if round >= 64 && encrypted.last().is_some_and(|&last| (last as usize) + 32 <= round) {
                break;
            }
        }
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&k[..32]);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_short_passwords_with_the_standard_string() {
        assert_eq!(padded(b""), PADDING);
        assert_eq!(&padded(b"ab")[..4], &[b'a', b'b', 0x28, 0xBF]);
        assert_eq!(padded(&[b'x'; 40]), [b'x'; 32]);
    }

    #[test]
    fn the_owner_password_recovers_the_user_password_it_was_built_from() {
        let parameters = LegacyParameters {
            revision: 3,
            key_length: 16,
            owner: &[],
            permissions: -4,
            first_id: b"id",
            encrypt_metadata: true,
        };
        let mut key = Md5::digest(padded(b"owner")).to_vec();
        for _ in 0..50 {
            key = Md5::digest(&key).to_vec();
        }
        let mut owner_value = padded(b"user").to_vec();
        for round in 0..=19u8 {
            owner_value = rc4(&xor_key(&key, round), &owner_value);
        }
        let parameters = LegacyParameters { owner: &owner_value, ..parameters };
        assert_eq!(legacy_user_from_owner(b"owner", &parameters), padded(b"user"));
    }
}
