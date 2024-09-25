//! Measure a FlowField generation for a world with a sparse sprinkling of impassable areas
//!
//! World is 100 sectors by 100 sectors
//!

use std::time::Duration;

use bevy::prelude::*;
use bevy_flowfield_tiles_plugin::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Create the required CostFields and Portals before benchmarking
fn prepare_fields(
	map_length: u32,
	map_depth: u32,
	sector_resolution: u32,
	actor_size: f32,
) -> (SectorPortals, SectorCostFields, MapDimensions, RouteCache) {
	let map_dimensions = MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
	let path =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/bench_costfields/heightmap_sparse.png";
	let cost_fields = SectorCostFields::from_heightmap(&map_dimensions, path);
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
		source_field_cell,
		target_sector,
		target_goal,
		Duration::default(),
		Route::new(path),
	);

	(portals, cost_fields, map_dimensions, route_cache)
}

/// Create the components of a FlowFieldTilesBundle and drive them with an actor in the top right
/// corner pathing to the bottom left
fn flow_sparse(
	portals: SectorPortals,
	cost_fields: SectorCostFields,
	map_dimensions: MapDimensions,
	route_cache: RouteCache,
) {
	let mut flow_cache = FlowFieldCache::default();
	// generate flow
	for (_key, portal_path) in route_cache.get_routes().iter() {
		// original order is from actor to goal, int fields need to be processed the other way around
		let mut route = portal_path.clone();
		route.get_mut().reverse();
		// create integration
		let mut int_builder = IntegrationBuilder::new(route.clone(), &cost_fields);
		int_builder.expand_field_portals(&portals, &cost_fields, &map_dimensions);
		int_builder.calculate_los();
		int_builder.build_integrated_cost(&cost_fields);
		// build the flow fields
		let sector_int_fields = int_builder.get_integration_fields();
		for (i, (sector_id, goals, int_field)) in sector_int_fields.iter().enumerate() {
			let mut flow_field = FlowField::default();
			// first element is end target, therefore has no info about previous sector for
			// direction optimisations
			if i == 0 {
				flow_field.calculate(goals, None, int_field);
				flow_cache.insert_field(
					*sector_id,
					route.get()[i].1,
					Duration::default(),
					flow_field,
				);
			} else if let Some(dir_prev_sector) =
				Ordinal::sector_to_sector_direction(sector_int_fields[i - 1].0, *sector_id)
			{
				let prev_int_field = &sector_int_fields[i - 1].2;
				flow_field.calculate(goals, Some((dir_prev_sector, prev_int_field)), int_field);
				//TODO by using the portal goal from path[i].1 actors criss-crossing from two seperate routes means one will use the others route in a sector which may be less efficient then using thier own
				flow_cache.insert_field(
					*sector_id,
					route.get()[i].1,
					Duration::default(),
					flow_field,
				);
			} else {
				error!("Route {:?}", portal_path);
			};
		}
	}
}

pub fn criterion_benchmark(c: &mut Criterion) {
	let mut group = c.benchmark_group("algorithm_use");
	group.significance_level(0.05).sample_size(100);
	let (portals, cost_fields, map_dimensions, route_cache) = prepare_fields(1000, 1000, 10, 0.5);
	group.bench_function("calc_flow_sparse", |b| {
		b.iter(|| {
			flow_sparse(
				black_box(portals.clone()),
				black_box(cost_fields.clone()),
				black_box(map_dimensions),
				black_box(route_cache.clone()),
			)
		})
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
