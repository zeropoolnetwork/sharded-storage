use p3_field::{AbstractField, PrimeField32, PrimeField64};
use p3_mersenne_31::Mersenne31;

const MASK: u32 = 0x3FFFFFFF;
const BITS_PER_ELEMENT: usize = 30;

pub fn encode(data: &[u8]) -> Vec<Mersenne31> {
    let mut result = Vec::new();
    let mut buffer = 0u32;
    let mut bits_in_buffer = 0;

    for &byte in data {
        buffer |= (byte as u32) << bits_in_buffer;
        bits_in_buffer += 8;

        if bits_in_buffer >= BITS_PER_ELEMENT {
            result.push(Mersenne31::from_canonical_u32(buffer & MASK));
            buffer >>= BITS_PER_ELEMENT;
            bits_in_buffer -= BITS_PER_ELEMENT;
        }
    }

    if bits_in_buffer > 0 {
        result.push(Mersenne31::from_canonical_u32(buffer & MASK));
    }

    result
}

pub fn decode(elements: &[Mersenne31], data_size: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(data_size);
    let mut buffer = 0u32;
    let mut bits_in_buffer = 0;

    for &element in elements {
        buffer |= element.as_canonical_u32() << bits_in_buffer;
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
}