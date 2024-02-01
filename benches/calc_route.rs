//! Measure calculating a route from one sector to another
//!
//! World is 100 sectors by 100 sectors
//!

use std::time::Duration;

use bevy_flowfield_tiles_plugin::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Create the required CostFields and Portals before benchmarking
fn prepare_fields(
	map_length: u32,
	map_depth: u32,
	sector_resolution: u32,
	actor_size: f32,
) -> (SectorPortals, SectorCostFields, PortalGraph) {
	let map_dimensions = MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
	let cost_fields = SectorCostFields::new(&map_dimensions);
	let mut portals = SectorPortals::new(
		map_dimensions.get_length(),
		map_dimensions.get_depth(),
		map_dimensions.get_sector_resolution(),
	);
	// update default portals for cost fields
	for sector_id in cost_fields.get_scaled().keys() {
		portals.update_portals(*sector_id, &cost_fields, &map_dimensions);
	}
	let graph = PortalGraph::new(&portals, &cost_fields, &map_dimensions);
	(portals, cost_fields, graph)
}

/// Create the components of a FlowFieldTilesBundle and drive them with an actor in the top right
/// corner pathing to the bottom left
fn calc(portals: SectorPortals, cost_fields: SectorCostFields, graph: PortalGraph) {
	let mut route_cache = RouteCache::default();

	// top right
	let source_sector = SectorID::new(99, 0);
	let source_field_cell = FieldCell::new(9, 0);
	let source = (source_sector, source_field_cell);
	// bottom left
	let target_sector = SectorID::new(0, 99);
	let target_goal = FieldCell::new(0, 9);
	let target = (target_sector, target_goal);

	// find the route
	let mut path = graph
		.find_best_path(source, target, &portals, &cost_fields)
		.unwrap();
	filter_path(&mut path, target_goal);

	route_cache.insert_route(
		source_sector,
		target_sector,
		target_goal,
		Duration::default(),
		path,
	);
}

pub fn criterion_benchmark(c: &mut Criterion) {
	let mut group = c.benchmark_group("algorithm_use");
	group.significance_level(0.05).sample_size(100);
	let (portals, cost_fields, graph) = prepare_fields(1000, 1000, 10, 0.5);
	group.bench_function("calc_route", |b| {
		b.iter(|| {
			calc(
				black_box(portals.clone()),
				black_box(cost_fields.clone()),
				black_box(graph.clone()),
			)
		})
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
