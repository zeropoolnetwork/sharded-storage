use criterion::{black_box, criterion_group, criterion_main, Criterion};
use primitives::{poseidon2_compress_hashes, Val, Hash};
use rand::{thread_rng, Rng};


type HashVal = [Val; 8];
fn bench_hash(left: HashVal, right: HashVal) -> Hash {

    let hashes = [left, right];
    poseidon2_compress_hashes(hashes)
}



fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("poseidon2");
    let mut rng = thread_rng();
    
    // Generate random input data
    let left = rng.gen();
    let right = rng.gen();

    group.bench_function("hash_pair", |b| {
        b.iter(|| bench_hash(black_box(left), black_box(right)))
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);