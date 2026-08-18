//! Measure calculating a route from one sector to another
//!
//! World is 100 sectors by 100 sectors
//!

use bevy::{app::App, math::Vec2};
use bevy_flowfield_tiles_plugin::v2::{bundle::FlowFieldTiles, plugin::FlowFieldTilesPlugin};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Create FlowFieldTiles
fn prepare(
	origin: (f32, f32),
	size: (f32, f32),
	world_unit_size: f32,
	actor_size: f32,
) -> FlowFieldTiles {
	FlowFieldTiles::new(origin, size, world_unit_size, actor_size)
}

/// Drive the algorithm to create a portal-portal route
fn calc(flowfield_tiles: FlowFieldTiles) {
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
	group.bench_function("calc_route", |b| {
		b.iter(|| {
			let flowfield_tiles = prepare(
				black_box((0.0, 0.0)),
				black_box((1000.0, 1000.0)),
				black_box(1.0),
				black_box(0.5),
			);
			calc(black_box(flowfield_tiles))
		})
	});
	group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
