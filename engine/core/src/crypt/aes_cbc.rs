use aes::cipher::{Array, BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};
use aes::{Aes128, Aes256};

use crate::error::{Error, Result};

enum Cipher {
    Aes128(Box<Aes128>),
    Aes256(Box<Aes256>),
}

impl Cipher {
    fn new(key: &[u8]) -> Result<Cipher> {
        match key.len() {
            16 => {
                Ok(Cipher::Aes128(Box::new(Aes128::new(&Array::try_from(key).map_err(|_| Error::new("bad AES key"))?))))
            }
            32 => {
                Ok(Cipher::Aes256(Box::new(Aes256::new(&Array::try_from(key).map_err(|_| Error::new("bad AES key"))?))))
            }
            other => Err(Error::new(format!("AES key of {other} bytes"))),
        }
    }

    fn encrypt(&self, block: &mut [u8; 16]) {
        let mut array = Array::from(*block);
        match self {
            Cipher::Aes128(cipher) => cipher.encrypt_block(&mut array),
            Cipher::Aes256(cipher) => cipher.encrypt_block(&mut array),
        }
        block.copy_from_slice(&array);
    }

    fn decrypt(&self, block: &mut [u8; 16]) {
        let mut array = Array::from(*block);
        match self {
            Cipher::Aes128(cipher) => cipher.decrypt_block(&mut array),
            Cipher::Aes256(cipher) => cipher.decrypt_block(&mut array),
        }
        block.copy_from_slice(&array);
    }
}

pub fn encrypt_blocks(key: &[u8], iv: [u8; 16], data: &[u8]) -> Result<Vec<u8>> {
    let cipher = Cipher::new(key)?;
    let mut previous = iv;
    let mut out = Vec::with_capacity(data.len());
    for chunk in data.chunks(16) {
        let mut block = [0u8; 16];
        block[..chunk.len()].copy_from_slice(chunk);
        for (byte, chained) in block.iter_mut().zip(previous) {
            *byte ^= chained;
        }
        cipher.encrypt(&mut block);
        out.extend_from_slice(&block);
        previous = block;
    }
    Ok(out)
}

pub fn decrypt_blocks(key: &[u8], iv: [u8; 16], data: &[u8]) -> Result<Vec<u8>> {
    let cipher = Cipher::new(key)?;
    let mut previous = iv;
    let mut out = Vec::with_capacity(data.len());
    for &chunk in data.as_chunks::<16>().0 {
        let mut block = chunk;
        let cipher_block = block;
        cipher.decrypt(&mut block);
        for (byte, chained) in block.iter_mut().zip(previous) {
            *byte ^= chained;
        }
        out.extend_from_slice(&block);
        previous = cipher_block;
    }
    Ok(out)
}

pub fn decrypt_with_iv(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 16 {
        return Ok(Vec::new());
    }
    let iv: [u8; 16] = data[..16].try_into().map_err(|_| Error::new("AES IV"))?;
    let mut plain = decrypt_blocks(key, iv, &data[16..data.len() - (data.len() - 16) % 16])?;
    if let Some(&padding) = plain.last()
        && (1..=16).contains(&padding)
        && plain.len() >= padding as usize
    {
        plain.truncate(plain.len() - padding as usize);
    }
    Ok(plain)
}

pub fn encrypt_with_iv(key: &[u8], iv: [u8; 16], data: &[u8]) -> Result<Vec<u8>> {
    let padding = 16 - data.len() % 16;
    let mut padded = data.to_vec();
    padded.extend(std::iter::repeat_n(padding as u8, padding));
    let mut out = iv.to_vec();
    out.extend(encrypt_blocks(key, iv, &padded)?);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_nist_cbc_vector() {
        let key = [0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c];
        let iv: [u8; 16] = std::array::from_fn(|index| index as u8);
        let plain = [0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a];
        let expected = [0x76, 0x49, 0xab, 0xac, 0x81, 0x19, 0xb2, 0x46, 0xce, 0xe9, 0x8e, 0x9b, 0x12, 0xe9, 0x19, 0x7d];
        assert_eq!(encrypt_blocks(&key, iv, &plain).unwrap(), expected);
        assert_eq!(decrypt_blocks(&key, iv, &expected).unwrap(), plain);
    }

    #[test]
    fn pads_and_strips_around_a_leading_iv() {
        let key = [7u8; 32];
        let sealed = encrypt_with_iv(&key, [9; 16], b"seventeen bytes!!").unwrap();
        assert_eq!(sealed.len(), 16 + 32);
        assert_eq!(decrypt_with_iv(&key, &sealed).unwrap(), b"seventeen bytes!!");
    }
}
