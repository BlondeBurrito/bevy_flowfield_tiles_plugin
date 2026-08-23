//! Measure calculating a route from one sector to another
//!
//! World is 100 sectors by 100 sectors
//!

use bevy::prelude::*;
use bevy_flowfield_tiles_plugin::prelude::*;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

/// Create FlowFieldTiles
fn prepare(
	origin: (f32, f32),
	size: (f32, f32),
	world_unit_size: f32,
	actor_radius: f32,
) -> FlowFieldTiles {
	FlowFieldTiles::new(origin, size, world_unit_size, actor_radius)
}

/// Find the route steps
fn calc(flowfield_tiles: &FlowFieldTiles) {
	let from = Vec2::new(499.5, 499.5);
	let to = Vec2::new(-499.5, -499.5);

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

	let Some(_) = portals_read.find_path(
		&source_sector,
		&source_cell,
		&goal_sector,
		&goal_cell,
		&read_costfields,
	) else {
		panic!("");
	};
}

pub fn criterion_benchmark(c: &mut Criterion) {
	let mut group = c.benchmark_group("algorithm_use");
	group.significance_level(0.05).sample_size(10);
	group.bench_function("calc_route", |b| {
		let flowfield_tiles = prepare(
			black_box((0.0, 0.0)),
			black_box((1000.0, 1000.0)),
			black_box(1.0),
			black_box(0.5),
		);

		b.iter(|| {
			calc(&flowfield_tiles);
		})
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
