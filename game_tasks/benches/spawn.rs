use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use game_tasks::TaskPool;

const COUNTS: &[usize] = &[1, 10, 100];

async fn do_work() -> i32 {
    let val = 1 + 1;
    black_box(val)
}

fn spawn_basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawn_basic");
    for &count in COUNTS {
        let id = BenchmarkId::new("spawn_basic", count);
        group.bench_with_input(id, &count, |b, &count| {
            b.iter_batched_ref(
                || TaskPool::new(1),
                |executor| {
                    let mut tasks = Vec::with_capacity(count);
                    for _ in 0..count {
                        tasks.push(executor.spawn(do_work()));
                    }

                    for task in tasks {
                        assert_eq!(futures::executor::block_on(task), 2);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }
}

criterion_group! {
    benches,
    spawn_basic,
}

criterion_main!(benches);
