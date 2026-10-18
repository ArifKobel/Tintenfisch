use sha1::Digest;

const BLOCK: usize = 64;

#[derive(Clone, Copy)]
pub enum KeyPurpose {
    Key = 1,
    Iv = 2,
    Mac = 3,
}

pub fn bmp_password(password: &str) -> Vec<u8> {
    password.encode_utf16().flat_map(u16::to_be_bytes).chain([0, 0]).collect()
}

pub fn derive<D: Digest>(password: &[u8], salt: &[u8], purpose: KeyPurpose, iterations: u32, length: usize) -> Vec<u8> {
    let stretched = |source: &[u8]| -> Vec<u8> {
        let size = source.len().div_ceil(BLOCK) * BLOCK;
        source.iter().copied().cycle().take(size).collect()
    };
    let diversifier = [purpose as u8; BLOCK];
    let mut input = stretched(salt);
    input.extend(stretched(password));
    let mut output = Vec::with_capacity(length);
    while output.len() < length {
        let mut hash = D::new().chain_update(diversifier).chain_update(&input).finalize().to_vec();
        for _ in 1..iterations.max(1) {
            hash = D::digest(&hash).to_vec();
        }
        let filler: Vec<u8> = hash.iter().copied().cycle().take(BLOCK).collect();
        for block in input.chunks_mut(BLOCK) {
            let mut carry = 1u16;
            for (byte, add) in block.iter_mut().zip(&filler).rev() {
                let sum = *byte as u16 + *add as u16 + carry;
                *byte = sum as u8;
                carry = sum >> 8;
            }
        }
        output.extend(hash);
    }
    output.truncate(length);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha1::Sha1;

    #[test]
    fn matches_the_published_sha1_vectors() {
        let password = bmp_password("smeg");
        let salt = [0x0a, 0x58, 0xcf, 0x64, 0x53, 0x0d, 0x82, 0x3f];
        let key = derive::<Sha1>(&password, &salt, KeyPurpose::Key, 1, 24);
        let iv = derive::<Sha1>(&password, &salt, KeyPurpose::Iv, 1, 8);
        let hex = |bytes: &[u8]| bytes.iter().map(|byte| format!("{byte:02X}")).collect::<String>();
        assert_eq!(hex(&key), "8AAAE6297B6CB04642AB5B077851284EB7128F1A2A7FBCA3");
        assert_eq!(hex(&iv), "79993DFE048D3B76");
    }
}
