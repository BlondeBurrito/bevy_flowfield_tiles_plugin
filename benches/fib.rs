//!
//!

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fib(n: u64) -> u64 {
	match n {
		0 => 1,
		1 => 1,
		n => fib(n-1) + fib(n-2),
	}
}

pub fn criterion_benchmark(c: &mut Criterion) {
	c.bench_function("fib 20", |b| b.iter(|| fib(black_box(20))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
