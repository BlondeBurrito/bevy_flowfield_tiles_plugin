//! Helper for setting up camera
//!

use bevy::prelude::*;

/// Get 2d camera setup with `scale` projection
#[allow(dead_code)]
pub fn get_camera_2d(scale: f32) -> (Camera2d, Projection) {
	let proj = Projection::Orthographic(OrthographicProjection {
		scale: scale,
		..OrthographicProjection::default_2d()
	});
	(Camera2d, proj)
}

/// Get a 3d camera
#[allow(dead_code)]
pub fn get_camera_3d() -> (Camera3d, Transform) {
	let mut tform = Transform::from_translation(Vec3::new(0.0, 40.0, 2.0));
	tform.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
	(Camera3d::default(), tform)
}
