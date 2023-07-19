//! Measure initialising the FlowFieldTilesBundle - this means that Portals and
//! the PortalGraph are calculated
//!

use bevy_flowfield_tiles_plugin::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Create a set of CostFields
fn init_bundle(map_length: u32, map_depth: u32) {
	// let map_dimensions = MapDimensions::new(map_length, map_depth);
	// 1000x1000 sectors
	let _ = FlowFieldTilesBundle::new(map_length, map_depth);
}

pub fn criterion_benchmark(c: &mut Criterion) {
	let mut group = c.benchmark_group("smaller_sample");
	group.significance_level(0.1).sample_size(10);
	group.bench_function("init_bundle", |b| {
		b.iter(|| init_bundle(black_box(1000), black_box(1000)))
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
