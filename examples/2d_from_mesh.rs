//! From a list of navigatable meshes construct the Flowfields. The pathable
//! (cost 1) FieldCells of each sector are shown as purple squares.
//!
//! Note that if two meshes overlap/touch then the edge between them will be
//! marked as impassable.
//!

//TODO figure out way to "bridge" adhacent meshes to create a contiguous pathable area

use bevy::{
	prelude::*,
	render::{
		mesh::{Indices, PrimitiveTopology},
		render_asset::RenderAssetUsages,
	},
	sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use examples_utils::_2d::create_wall_colliders;

use avian2d::prelude::*;
use bevy_flowfield_tiles_plugin::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			DefaultPlugins,
			PhysicsPlugins::default(),
			// PhysicsDebugPlugin::default(),
		))
		.insert_resource(SubstepCount(30))
		.insert_resource(Gravity(Vec2::ZERO))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(Startup, (setup, create_wall_colliders, create_meshes))
		.add_systems(
			Update,
			(create_flowfields_from_meshes, mark_pathable_field_cells),
		)
		.run();
}

/// Init
fn setup(mut cmds: Commands) {
	let mut camera = Camera2dBundle::default();
	camera.projection.scale = 2.0;
	cmds.spawn(camera);
}

/// Labels meshes to be used to initialise a [FlowFieldTilesBundle]
#[derive(Component)]
struct Pathable;

/// Create two meshes in the shape of a 'T' which include the component [Pathable] to indicate we should use them when supply meshes to generate the Flowfields, and create
fn create_meshes(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<ColorMaterial>>,
) {
	let upper_t = Mesh::new(
		PrimitiveTopology::TriangleList,
		RenderAssetUsages::default(),
	)
	.with_inserted_attribute(
		Mesh::ATTRIBUTE_POSITION,
		vec![
			[-960.0, 640.0, 0.0],
			[-960.0, 960.0, 0.0],
			[700.0, 960.0, 0.0],
			[900.0, 800.0, 0.0],
			[700.0, 640.0, 0.0],
		],
	)
	.with_inserted_indices(Indices::U32(vec![0, 1, 2, 2, 3, 4, 4, 2, 0]));

	cmds.spawn((
		MaterialMesh2dBundle {
			mesh: meshes.add(upper_t).into(),
			material: materials.add(Color::WHITE),
			transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
			..default()
		},
		Pathable,
	));

	let lower_t = Mesh::new(
		PrimitiveTopology::TriangleList,
		RenderAssetUsages::default(),
	)
	.with_inserted_attribute(
		Mesh::ATTRIBUTE_POSITION,
		vec![
			[-320.0, -960.0, 0.0],
			[-320.0, 640.0, 0.0],
			[320.0, 640.0, 0.0],
			[320.0, -960.0, 0.0],
		],
	)
	.with_inserted_indices(Indices::U32(vec![0, 1, 2, 2, 0, 3]));
	cmds.spawn((
		MaterialMesh2dBundle {
			mesh: meshes.add(lower_t).into(),
			material: materials.add(Color::WHITE),
			transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
			..default()
		},
		Pathable,
	));
	let island = Mesh::new(
		PrimitiveTopology::TriangleStrip,
		RenderAssetUsages::default(),
	)
	.with_inserted_attribute(
		Mesh::ATTRIBUTE_POSITION,
		vec![
			[-192.0, 640.0, 0.0],
			[-192.0, -640.0, 0.0],
			[192.0, 640.0, 0.0],
			[192.0, -640.0, 0.0],
		],
	)
	.with_inserted_indices(Indices::U32(vec![0, 1, 2, 3]));
	cmds.spawn((
		MaterialMesh2dBundle {
			mesh: meshes.add(island).into(),
			material: materials.add(Color::WHITE),
			transform: Transform::from_translation(Vec3::new(-580.0, -64.0, 0.0)),
			..default()
		},
		Pathable,
	));
	//TODO create collider meshes around the T?
}

/// Once the meshes are ready use them to create a [FlowFieldTilesBundle]
fn create_flowfields_from_meshes(
	mut cmds: Commands,
	query: Query<(&Mesh2dHandle, &Transform), With<Pathable>>,
	meshes: Res<Assets<Mesh>>,
	mut is_complete: Local<bool>,
) {
	if !*is_complete {
		// ensure meshes are ready
		let expected_ready_meshes_count = 3;
		let mut ready_meshes_count = 0;
		let mut pathable_meshes = vec![];
		for (handle, tform) in &query {
			if let Some(mesh) = meshes.get(&handle.0) {
				ready_meshes_count += 1;
				pathable_meshes.push((mesh, tform.translation.truncate()));
			}
		}
		// create the flowfields
		if ready_meshes_count == expected_ready_meshes_count {
			let mut meshes = vec![];
			for (m, t) in pathable_meshes {
				meshes.push((m, t));
			}
			let map_length = 1920;
			let map_depth = 1920;
			let sector_resolution = 320;
			let actor_size = 16.0;
			cmds.spawn(FlowFieldTilesBundle::from_bevy_2d_meshes(
				meshes,
				map_length,
				map_depth,
				sector_resolution,
				actor_size,
				1,
				255,
			));
			*is_complete = true;
		}
	}
}

/// Create purple squares to show where pathable FieldCells have ben calcualted
/// from the input meshes
fn mark_pathable_field_cells(
	mut cmds: Commands,
	query: Query<(&SectorCostFields, &MapDimensions)>,
	mut is_complete: Local<bool>,
) {
	if !*is_complete {
		for (sector_costfields, map_dimensions) in &query {
			for (sector, field) in sector_costfields.get_baseline().iter() {
				let array = field.get();
				for (i, column) in array.iter().enumerate() {
					for (j, value) in column.iter().enumerate() {
						if *value == 1 {
							let field_cell = FieldCell::new(i, j);
							if let Some(pos) =
								map_dimensions.get_xy_from_field_sector(*sector, field_cell)
							{
								cmds.spawn(SpriteBundle {
									sprite: Sprite {
										color: Color::srgb(230.0, 0.0, 255.0),
										..default()
									},
									transform: Transform {
										translation: pos.extend(1.0),
										scale: Vec3::new(32.0, 32.0, 1.0),
										..default()
									},
									..default()
								});
							}
						}
					}
				}
			}
			*is_complete = true;
		}
	}
}
