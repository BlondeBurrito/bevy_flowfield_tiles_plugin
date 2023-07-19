//! Measure initialising a large set of CostFields
//!

use bevy_flowfield_tiles_plugin::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Create a set of CostFields
fn init_cost_fields(map_length: u32, map_depth: u32) {
	// let map_dimensions = MapDimensions::new(map_length, map_depth);
	// 1000x1000 sectors
	let _cost_fields = SectorCostFields::new(map_length, map_depth);
}

pub fn criterion_benchmark(c: &mut Criterion) {
	let mut group = c.benchmark_group("smaller_sample");
	group.significance_level(0.05).sample_size(100);
	group.bench_function("init_sector_cost_fields", |b| {
		b.iter(|| init_cost_fields(black_box(1000), black_box(1000)))
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
