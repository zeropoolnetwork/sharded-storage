use p3_mersenne_31::Mersenne31;
use p3_field::PrimeField32;
use p3_poseidon2::poseidon2_round_numbers_128;


use rand::Rng;
use rand::SeedableRng;
use rand::distributions::Standard; 
use rand_chacha::ChaCha20Rng;

use sha3::{Digest, Keccak256};



fn seed(input: &str) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    output
}


fn main() {
    let rng = &mut ChaCha20Rng::from_seed(seed("ZeroPool"));

    type Val = Mersenne31;





    const WIDTH: usize = 16;
    const D: u64 = 5;


    let (rounds_f, rounds_p) = poseidon2_round_numbers_128::<Val>(WIDTH, D);

    
    let external_constants = rng.sample_iter(Standard).take(rounds_f).collect::<Vec<[Val; WIDTH]>>();
    
    let internal_constants = rng.sample_iter(Standard).take(rounds_p).collect::<Vec<Val>>();

    println!("pub const POSEIDON2_W_{}_D_{}_ROUNDS_F: usize = {};", WIDTH, D, rounds_f);
    println!("pub const POSEIDON2_W_{}_D_{}_ROUNDS_P: usize = {};", WIDTH, D, rounds_p);

    println!("pub const POSEIDON2_W_{}_D_{}_EXTERNAL_CONSTANTS: [[Mersenne31; {}]; {}] = [", WIDTH, D, WIDTH, rounds_f);
    for round in external_constants.iter() {
        print!("    [");
        for (i, &val) in round.iter().enumerate() {
            if i > 0 {
                if i % 4 == 0 {
                    print!(",\n        ");
                } else {
                    print!(", ");
                }
            }
            print!("Mersenne31::new({})", val.as_canonical_u32());
        }
        println!("],");
    }
    println!("];");

    println!("pub const POSEIDON2_W_{}_D_{}_INTERNAL_CONSTANTS: [Mersenne31; {}] = [", WIDTH, D, rounds_p);
    for (i, &val) in internal_constants.iter().enumerate() {
        if i > 0 {
            if i % 4 == 0 {
                print!(",\n    ");
            } else {
                print!(", ");
            }
        }
        print!("Mersenne31::new({})", val.as_canonical_u32());
    }
    println!("\n];");


}
