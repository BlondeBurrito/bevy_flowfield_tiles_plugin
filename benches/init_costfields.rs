//! Measure initialising a large set of CostFields
//!

use bevy_flowfield_tiles_plugin::prelude::*;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

/// Create a set of CostFields
fn init_cost_fields(origin: (f32, f32), size: (f32, f32), world_unit_size: f32, actor_radius: f32) {
	let dimensions = Dimensions::new(origin, size, world_unit_size, actor_radius);
	let _cost_fields = SectorCostFields::new(&dimensions);
}

pub fn criterion_benchmark(c: &mut Criterion) {
	let mut group = c.benchmark_group("data_initialisation");
	group.significance_level(0.05).sample_size(50);
	group.bench_function("init_costfields", |b| {
		b.iter(|| {
			init_cost_fields(
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
