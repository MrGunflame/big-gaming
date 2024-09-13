use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use game_common::collections::lru::LruCache;

fn run_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("insertion");
    for size in [1024, 4096, 8192] {
        let id = BenchmarkId::new("lru insertion", size);

        group.bench_with_input(id, &size, |b, &size| {
            b.iter_batched_ref(
                || LruCache::new(8192),
                |cache| {
                    for index in 0..size {
                        cache.insert(index, index);
                    }
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();

    let mut group = c.benchmark_group("read");
    for size in [1024, 4096, 8192] {
        let id = BenchmarkId::new("lru read", size);

        group.bench_with_input(id, &size, |b, &size| {
            b.iter_batched_ref(
                || {
                    let mut cache = LruCache::new(8192);
                    for index in 0..size {
                        cache.insert(index, index);
                    }
                    cache
                },
                |cache| {
                    for index in 0..size {
                        cache.get(&index);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, run_bench);
criterion_main!(benches);
