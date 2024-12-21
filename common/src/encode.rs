use p3_field::{AbstractField, PrimeField32};
use p3_mersenne_31::Mersenne31;

#[derive(Debug)]
pub enum Error {
    InvalidDataSize,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InvalidDataSize => write!(f, "Invalid data size"),
        }
    }
}

impl std::error::Error for Error {}

const MASK: u64 = 0x3FFFFFFF;
const BITS_PER_ELEMENT: usize = 30;

pub fn encode(data: &[u8]) -> Vec<Mersenne31> {
    let mut result = Vec::new();
    let mut buffer = 0u64;
    let mut bits_in_buffer = 0;

    for &byte in data {
        buffer |= (byte as u64) << bits_in_buffer;
        bits_in_buffer += 8;

        if bits_in_buffer >= BITS_PER_ELEMENT {
            result.push(Mersenne31::from_canonical_u32((buffer & MASK) as u32));
            buffer >>= BITS_PER_ELEMENT;
            bits_in_buffer -= BITS_PER_ELEMENT;
        }
    }

    if bits_in_buffer > 0 {
        result.push(Mersenne31::from_canonical_u32((buffer & MASK) as u32));
    }

    result
}

pub fn encode_aligned(data: &[u8], n_elements: usize) -> Result<Vec<Mersenne31>, Error> {
    if data.len() > n_elements * 30 / 8 {
        return Err(Error::InvalidDataSize);
    }

    let mut result = Vec::with_capacity(n_elements);
    result.extend(
        // TODO: get rid of the extra allocation
        encode(data)
            .into_iter()
            .chain((0..).map(|_| Mersenne31::zero()))
            .take(n_elements),
    );

    Ok(result)
}

pub fn decode(elements: &[Mersenne31], data_size: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(data_size);
    let mut buffer = 0u64;
    let mut bits_in_buffer = 0;

    for &element in elements {
        buffer |= (element.as_canonical_u32() as u64) << bits_in_buffer;
        bits_in_buffer += BITS_PER_ELEMENT;

        while bits_in_buffer >= 8 && result.len() < data_size {
            result.push((buffer & 0xFF) as u8);
            buffer >>= 8;
            bits_in_buffer -= 8;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let bytes = b"1234567890-=[qwertyuiop[]asdfghjkl;'zxcvbnm,./";
        let encoded = encode(bytes);

        let decoded = decode(&encoded, bytes.len());
        assert_eq!(bytes, decoded.as_slice());
    }

    #[test]
    fn test_encode() {
        let bytes = [
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        ];
        let encoded = encode(&bytes);
        let decoded = decode(&encoded, bytes.len());
        assert_eq!(bytes, decoded.as_slice());
    }
}
