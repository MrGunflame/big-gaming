use std::future::Future;
use std::task::Poll;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use futures_lite::future::poll_fn;
use game_tasks::TaskPool;

const COUNTS: &[usize] = &[1, 10, 100];

async fn do_work() -> i32 {
    let val = 1 + 1;
    black_box(val)
}

fn yield_once() -> impl Future<Output = i32> {
    let val = 1 + 1;
    let mut yielded = false;

    poll_fn(move |cx| {
        if yielded {
            return Poll::Ready(black_box(val));
        }

        yielded = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    })
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
                        assert_eq!(futures_lite::future::block_on(task), 2);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }
}

fn spawn_yield_once(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawn_yield_once");
    for &count in COUNTS {
        let id = BenchmarkId::new("spawn_yield_once", count);
        group.bench_with_input(id, &count, |b, &count| {
            b.iter_batched_ref(
                || TaskPool::new(1),
                |executor| {
                    let mut tasks = Vec::with_capacity(count);
                    for _ in 0..count {
                        tasks.push(executor.spawn(yield_once()));
                    }

                    for task in tasks {
                        assert_eq!(futures_lite::future::block_on(task), 2);
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
    spawn_yield_once,
}

criterion_main!(benches);
