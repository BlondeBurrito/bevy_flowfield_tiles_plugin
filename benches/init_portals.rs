//! Measure calculating Portals
//!

use bevy_flowfield_tiles_plugin::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Create the required CostFields before benchmarking
fn prepare_fields(
	map_length: u32,
	map_depth: u32,
	sector_resolution: u32,
	actor_size: f32,
) -> (SectorCostFieldsScaled, MapDimensions) {
	let map_dimensions = MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
	let cost_fields = SectorCostFields::new(map_length, map_depth, sector_resolution);
	let cost_fields_scaled =
		SectorCostFieldsScaled::new(&cost_fields, map_dimensions.get_actor_scale());
	(cost_fields_scaled, map_dimensions)
}

/// Create a set of CostFields
fn init_portals(cost_fields_scaled: SectorCostFieldsScaled, map_dimensions: MapDimensions) {
	let mut portals = SectorPortals::new(
		map_dimensions.get_length(),
		map_dimensions.get_depth(),
		map_dimensions.get_sector_resolution(),
	);
	// update default portals for cost fields
	for sector_id in cost_fields_scaled.get().keys() {
		portals.update_portals(*sector_id, &cost_fields_scaled, &map_dimensions);
	}
}

pub fn criterion_benchmark(c: &mut Criterion) {
	let mut group = c.benchmark_group("data_initialisation");
	group.significance_level(0.05).sample_size(100);
	let (cost_fields_scaled, map_dimensions) = prepare_fields(1000, 1000, 10, 0.5);
	group.bench_function("init_portals", |b| {
		b.iter(|| {
			init_portals(
				black_box(cost_fields_scaled.clone()),
				black_box(map_dimensions),
			)
		})
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
