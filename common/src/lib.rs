use p3_mersenne_31::Mersenne31;

pub mod encode;
pub mod config;

pub fn blowup(data: &[Mersenne31], factor: usize) -> Vec<Mersenne31> {
    let mut result = Vec::with_capacity(data.len() * factor);

    for &element in data {
        for _ in 0..factor - 1 {
            result.push(element);
        }
    }

    result
}