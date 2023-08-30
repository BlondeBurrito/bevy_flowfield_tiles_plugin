//! 
//! 

// use bevy::prelude::*;

// #[derive(Component)]
// struct Collider;

// #[derive(Component)]
// struct Boundary;

// #[derive(Bundle)]
// struct BoundaryBundle {
// 	sprite_bundle: SpriteBundle,
// 	boundary: Boundary,
// 	collider: Collider,
// }

// impl BoundaryBundle {
// 	pub fn new_set(dimension: f32) -> Vec<BoundaryBundle> {
// 		let top_location = Vec3::new(0.0, dimension * 15.0, 0.0);
// 	let top_scale = Vec3::new(dimension * 30.0, dimension, 1.0);
// 	let bottom_location = Vec3::new(0.0, -dimension * 15.0, 0.0);
// 	let bottom_scale = Vec3::new(dimension * 30.0, dimension, 1.0);
// 	let left_location = Vec3::new(-dimension * 15.0, 0.0, 0.0);
// 	let left_scale = Vec3::new(dimension, dimension * 30.0, 1.0);
// 	let right_location = Vec3::new(dimension * 15.0, 0.0, 0.0);
// 	let right_scale = Vec3::new(dimension, dimension * 30.0, 1.0);

// 	let walls = vec![
// 		(top_location, top_scale),
// 		(bottom_location, bottom_scale),
// 		(left_location, left_scale),
// 		(right_location, right_scale),
// 	];

// 	let mut bundles = Vec::new();

// 	for (loc, scale) in walls.iter() {
// 		bundles.push(BoundaryBundle {
// 			sprite_bundle: SpriteBundle {
// 				transform: Transform {
// 					translation: *loc,
// 					scale: *scale,
// 					..default()
// 				},
// 				sprite: Sprite {
// 					color: Color::BLACK,
// 					..default()
// 				},
// 				..default()
// 			},
// 			boundary: Boundary,
// 			collider: Collider,
// 		});
// 	}
// 	bundles
// 	}
// }


// /// Create collider entities around the world
// fn create_wall_colliders(mut cmds: Commands) {
// 	let top_location = Vec3::new(0.0, FIELD_SPRITE_DIMENSION * 15.0, 0.0);
// 	let top_scale = Vec3::new(FIELD_SPRITE_DIMENSION * 30.0, FIELD_SPRITE_DIMENSION, 1.0);
// 	let bottom_location = Vec3::new(0.0, -FIELD_SPRITE_DIMENSION * 15.0, 0.0);
// 	let bottom_scale = Vec3::new(FIELD_SPRITE_DIMENSION * 30.0, FIELD_SPRITE_DIMENSION, 1.0);
// 	let left_location = Vec3::new(-FIELD_SPRITE_DIMENSION * 15.0, 0.0, 0.0);
// 	let left_scale = Vec3::new(FIELD_SPRITE_DIMENSION, FIELD_SPRITE_DIMENSION * 30.0, 1.0);
// 	let right_location = Vec3::new(FIELD_SPRITE_DIMENSION * 15.0, 0.0, 0.0);
// 	let right_scale = Vec3::new(FIELD_SPRITE_DIMENSION, FIELD_SPRITE_DIMENSION * 30.0, 1.0);

// 	let walls = vec![
// 		(top_location, top_scale),
// 		(bottom_location, bottom_scale),
// 		(left_location, left_scale),
// 		(right_location, right_scale),
// 	];

// 	for (loc, scale) in walls.iter() {
// 		cmds.spawn((
// 			SpriteBundle {
// 				transform: Transform {
// 					translation: *loc,
// 					scale: *scale,
// 					..default()
// 				},
// 				sprite: Sprite {
// 					color: Color::BLACK,
// 					..default()
// 				},
// 				..default()
// 			},
// 			Collider,
// 		));
// 	}
// }