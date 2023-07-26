//! Measure calculating the PortalGraph
//!

use bevy_flowfield_tiles_plugin::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Create the required CostFields and Portals before benchmarking
fn prepare_fields(
	map_length: u32,
	map_depth: u32
	,sector_resolution: u32
) -> (SectorPortals, SectorCostFields, MapDimensions) {
	let map_dimensions = MapDimensions::new(map_length, map_depth, sector_resolution);
	let cost_fields = SectorCostFields::new(map_length, map_depth, sector_resolution);
	let mut portals = SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
	// update default portals for cost fields
	for sector_id in cost_fields.get().keys() {
		portals.update_portals(
			*sector_id,
			&cost_fields,
			&map_dimensions
		);
	}
	(portals, cost_fields, map_dimensions)
}

/// Create a set of CostFields
fn init_graph(
	portals: SectorPortals,
	cost_fields: SectorCostFields,
	map_dimensions: MapDimensions,
) {
	let _graph = PortalGraph::new(
		&portals,
		&cost_fields,
		&map_dimensions
	);
}

pub fn criterion_benchmark(c: &mut Criterion) {
	let mut group = c.benchmark_group("data_initialisation");
	group.significance_level(0.1).sample_size(10);
	let (portals, cost_fields, map_dimensions) = prepare_fields(1000, 1000, 10);
	group.bench_function("init_portal_graph", |b| {
		b.iter(|| {
			init_graph(
				black_box(portals.clone()),
				black_box(cost_fields.clone()),
				black_box(map_dimensions.clone()),
			)
		})
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
