//! Measure initialising the FlowFieldTiles
//!

use bevy_flowfield_tiles_plugin::prelude::*;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

/// Create FlowFieldTiles
fn init_bundle(origin: (f32, f32), size: (f32, f32), world_unit_size: f32, actor_radius: f32) {
	let _ = FlowFieldTiles::new(origin, size, world_unit_size, actor_radius);
}

pub fn criterion_benchmark(c: &mut Criterion) {
	let mut group = c.benchmark_group("data_initialisation");
	group.significance_level(0.1).sample_size(10);
	group.bench_function("init_bundle", |b| {
		b.iter(|| {
			init_bundle(
				black_box((0.0, 0.0)),
				black_box((1000.0, 1000.0)),
				black_box(1.0),
				black_box(0.5),
			)
		})
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
