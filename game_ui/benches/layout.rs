use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};

use game_ui::render::layout::LayoutTree;
use game_ui::render::{Element, ElementBody};
use game_ui::style::Style;

fn create_element() -> Element {
    Element {
        body: ElementBody::Container,
        style: Style::default(),
    }
}

fn build_layout_tree(children_per_elem: usize, depth: usize) -> LayoutTree {
    let mut tree = LayoutTree::new();

    let root = tree.push(None, create_element());

    let mut parents = vec![root];
    for _ in 0..depth {
        let iter_parents = parents.clone();
        parents.clear();

        for parent in iter_parents {
            for _ in 0..children_per_elem {
                parents.push(tree.push(Some(parent), create_element()));
            }
        }
    }

    tree
}

fn run_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout flat");
    for size in [1_000, 10_000, 100_000] {
        let id = BenchmarkId::new("layout flat", size);

        group.bench_with_input(id, &size, |b, &size| {
            b.iter_batched_ref(
                || build_layout_tree(size, 1),
                |tree| {
                    black_box(tree.compute_layout());
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();

    let mut group = c.benchmark_group("layout nested");
    for size in [100] {
        for depth in [3] {
            let id = BenchmarkId::new("layout nested", format!("{}/{}", size, depth));

            group.bench_with_input(id, &size, |b, &size| {
                b.iter_batched_ref(
                    || build_layout_tree(size, depth),
                    |tree| {
                        black_box(tree.compute_layout());
                    },
                    BatchSize::SmallInput,
                );
            });
        }
    }
    group.finish();

    let mut group = c.benchmark_group("layout super deep");
    for size in [100, 1_000] {
        let id = BenchmarkId::new("layout super deep", size);

        group.bench_with_input(id, &size, |b, &size| {
            b.iter_batched_ref(
                || build_layout_tree(1, size),
                |tree| {
                    black_box(tree.compute_layout());
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, run_bench);
criterion_main!(benches);
