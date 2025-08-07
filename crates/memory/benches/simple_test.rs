use criterion::{criterion_group, criterion_main, Criterion};

fn simple_benchmark(c: &mut Criterion) {
    c.bench_function("simple test", |b| {
        b.iter(|| {
            let x = 1 + 1;
            x
        })
    });
}

criterion_group!(benches, simple_benchmark);
criterion_main!(benches);
