pub fn decode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 2);
    let mut index = 0;
    while let Some(&length) = data.get(index) {
        index += 1;
        match length {
            0..=127 => {
                let end = (index + length as usize + 1).min(data.len());
                out.extend_from_slice(&data[index..end]);
                index = end;
            }
            128 => break,
            _ => {
                let Some(&byte) = data.get(index) else { break };
                out.extend(std::iter::repeat_n(byte, 257 - length as usize));
                index += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_literal_and_repeated_runs_until_the_end_marker() {
        assert_eq!(decode(&[2, b'a', b'b', b'c', 254, b'x', 128, 0, b'z']), b"abcxxx");
    }

    #[test]
    fn tolerates_a_truncated_run() {
        assert_eq!(decode(&[5, b'a', b'b']), b"ab");
        assert_eq!(decode(&[200]), b"");
    }
}
