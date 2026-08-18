//! Measure a FlowField generation for a world with a maze of impassable field cells.
//!
//! World is 100 sectors by 100 sectors with a snake-like maze of impassable cost field values running up and down the entire world - effectively a giant version of examples/2d_complex_movement
//!
//! ```txt
//!  _____________________________
//! |__|__|__|xx|__|__|__|xx|__|__|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|
//! |__|xx|__|xx|__|xx|__|xx|__|xx|
//! |__|xx|__|__|__|xx|__|__|__|xx|
//! ```
//!

use bevy::{prelude::*, tasks::futures::check_ready};
use bevy_flowfield_tiles_plugin::v2::{bundle::FlowFieldTiles, plugin::FlowFieldTilesPlugin};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Create FlowFieldTiles
fn prepare(
	origin: (f32, f32),
	size: (f32, f32),
	world_unit_size: f32,
	actor_size: f32,
) -> FlowFieldTiles {
	let path =
		env!("CARGO_MANIFEST_DIR").to_string() + "/assets/bench_costfields/heightmap_maze.png";
	FlowFieldTiles::from_heightmap(origin, size, world_unit_size, actor_size, &path)
}

/// Drive the algorithm and measures the time taken to compute all the required
/// FlowFields
fn flow_maze(flowfield_tiles: FlowFieldTiles) {
	let from = Vec2::new(499.5, 499.5);
	let to = Vec2::new(-499.5, -499.5);

	let op_task = flowfield_tiles.get_route_2d(from, to);

	// poll until route is ready
	let mut route_ready = false;
	while !route_ready {
		if let Some(task) = &op_task {
			if task.is_finished() {
				route_ready = true;
			}
		}
	}

	// verify all flows are computed
	let steps = check_ready(&mut op_task.unwrap()).unwrap().unwrap();
	let mut are_flows_ready = false;
	while !are_flows_ready {
		let req_flows = steps.len();
		let mut found_flows = 0;
		for step in steps.iter() {
			if flowfield_tiles.read_flowfield(step).is_some() {
				found_flows += 1;
			}
		}
		if found_flows == req_flows {
			are_flows_ready = true;
		}
	}
}

pub fn criterion_benchmark(c: &mut Criterion) {
	// require plugin to drive algorithm
	let mut app = App::new();
	app.add_plugins(FlowFieldTilesPlugin);
	//TODO
	let mut group = c.benchmark_group("algorithm_use");
	group.significance_level(0.05).sample_size(100);
	// let flowfield_tiles = prepare(
	// 	black_box((0.0, 0.0)),
	// 	black_box((1000.0, 1000.0)),
	// 	black_box(1.0),
	// 	black_box(0.5),
	// );
	group.bench_function("calc_flow_maze", |b| {
		b.iter(|| {
			let flowfield_tiles = prepare(
				black_box((0.0, 0.0)),
				black_box((1000.0, 1000.0)),
				black_box(1.0),
				black_box(0.5),
			);
			flow_maze(flowfield_tiles)
		})
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
