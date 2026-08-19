//! Measure calculating Portals
//!

use bevy_flowfield_tiles_plugin::prelude::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Create the required CostFields before benchmarking
fn prepare_fields(
	origin: (f32, f32),
	size: (f32, f32),
	world_unit_size: f32,
	actor_size: f32,
) -> SectorCostFields {
	let dimensions = Dimensions::new(origin, size, world_unit_size, actor_size);
	let cost_fields = SectorCostFields::new(&dimensions);
	cost_fields
}

/// Create a set of CostFields
fn init_portals(costfields: SectorCostFields) {
	let _portals = Portals::new(&costfields);
}

pub fn criterion_benchmark(c: &mut Criterion) {
	let mut group = c.benchmark_group("data_initialisation");
	group.significance_level(0.05).sample_size(100);
	let cost_fields = prepare_fields((0.0, 0.0), (1000.0, 1000.0), 1.0, 0.5);
	group.bench_function("init_portals", |b| {
		b.iter(|| init_portals(black_box(cost_fields.clone())))
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
