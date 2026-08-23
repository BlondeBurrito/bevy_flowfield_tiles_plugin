//! Measure a FlowField generation for a world with a maze of impassable field cells.
//!
//! World is 100 sectors by 100 sectors with a snake-like maze of impassable cost field values running up and down the entire world. The requested path goes through every sector multiple times
//!
//! ```txt
//!  ________________________________
//! |__|__|__|xx|__|__|__|xx|__|__|__|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|__|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|__|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|__|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|__|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|__|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|__|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|__|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|__|
//! |__|xx|__|__|__|xx|__|__|__|xx|__|
//! ```
//!

use bevy::prelude::*;
use bevy_flowfield_tiles_plugin::prelude::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Create FlowFieldTiles
fn prepare(
	origin: (f32, f32),
	size: (f32, f32),
	world_unit_size: f32,
	actor_radius: f32,
) -> (FlowFieldTiles, Vec<RouteStep>) {
	let file =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/bench_costfields/heightmap_maze.png";
	let flowfield_tiles =
		FlowFieldTiles::from_heightmap(origin, size, world_unit_size, actor_radius, &file);

	let from = Vec2::new(-499.5, -499.5);
	let to = Vec2::new(499.5, -499.5);

	let dimensions = flowfield_tiles.dimensions;
	let Some((source_sector, source_cell)) = dimensions.get_sector_and_field_cell_from_xy(from)
	else {
		panic!("");
	};
	let Some((goal_sector, goal_cell)) = dimensions.get_sector_and_field_cell_from_xy(to) else {
		panic!("");
	};

	let sector_costs = flowfield_tiles.sector_cost_fields.clone();
	let read_costfields = sector_costs.read().unwrap();

	let portals = flowfield_tiles.portals.clone();
	let portals_read = portals.read().unwrap();

	let Some(path) = portals_read.find_path(
		&source_sector,
		&source_cell,
		&goal_sector,
		&goal_cell,
		&read_costfields,
	) else {
		panic!("");
	};
	(flowfield_tiles, path)
}

/// Build the fields
fn calc(flowfield_tiles: &FlowFieldTiles, path: &Vec<RouteStep>) {
	let sector_costs = flowfield_tiles.sector_cost_fields.clone();
	let read_costfields = sector_costs.read().unwrap();

	for step in path.iter().rev() {
		let sector = step.get_sector();
		let scaled_costfields = read_costfields.get_scaled_costs();
		let scaled_costfield = scaled_costfields.get(sector).unwrap();

		let mut integrationfield = IntegrationField::init(scaled_costfield, step);
		integrationfield.build(scaled_costfield);

		let mut flowfield = FlowField::new(step, &integrationfield);
		flowfield.build(&integrationfield);
	}
}

pub fn criterion_benchmark(c: &mut Criterion) {
	let mut group = c.benchmark_group("algorithm_use");
	group.significance_level(0.05).sample_size(10);
	let (flowfield_tiles, path) = prepare(
		black_box((0.0, 0.0)),
		black_box((1000.0, 1000.0)),
		black_box(1.0),
		black_box(0.5),
	);
	group.bench_function("calc_flow_maze", |b| {
		b.iter(|| {
			calc(&flowfield_tiles, &path);
		})
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
