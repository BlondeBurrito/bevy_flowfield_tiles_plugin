//! Measure initialising a large set of CostFields
//!

use bevy_flowfield_tiles_plugin::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Create a set of CostFields
fn init_cost_fields(map_length: u32, map_depth: u32, sector_resolution: u32, actor_size: f32) {
	let map_dimensions = MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
	let _cost_fields = SectorCostFields::new(&map_dimensions);
}

pub fn criterion_benchmark(c: &mut Criterion) {
	let mut group = c.benchmark_group("data_initialisation");
	group.significance_level(0.05).sample_size(100);
	group.bench_function("init_sector_cost_fields", |b| {
		b.iter(|| {
			init_cost_fields(
				black_box(1000),
				black_box(1000),
				black_box(10),
				black_box(0.5),
			)
		})
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
