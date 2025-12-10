use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::Rng;

// Benchmarks the encoding speed for random input.
fn bench_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode");

    for size in [16, 256, 4096, 65536, 262144, 1048576, 4194304] {
        let data: Vec<u8> = rand::rng().random_iter().take(size).collect();
        let mut buffer = vec![0u8; cobs::max_encoding_length(size)];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| {
                cobs::encode(data, &mut buffer);
            });
        });
    }
    group.finish();
}

// Benchmarks the decoding speed for random input.
fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");

    for size in [16, 256, 4096, 65536, 262144, 1048576, 4194304] {
        let data: Vec<u8> = rand::rng().random_iter().take(size).collect();
        let mut encoded = vec![0u8; cobs::max_encoding_length(size)];

        let len = cobs::encode(&data, &mut encoded);
        encoded.truncate(len);

        let mut buffer = vec![0u8; size];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &encoded, |b, encoded| {
            b.iter(|| {
                cobs::decode(encoded, &mut buffer).unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_encode, bench_decode);
criterion_main!(benches);
