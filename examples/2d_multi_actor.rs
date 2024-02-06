//! Generates a 30x30 world where multiple actors can be told to move soomewhere with right click
//!

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_flowfield_tiles_plugin::prelude::*;
use bevy_xpbd_2d::prelude::*;

/// Dimension of square sprites making up the world
const FIELD_SPRITE_DIMENSION: f32 = 64.0;

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
		.add_systems(
			Startup,
			(setup_visualisation, setup_navigation, create_wall_colliders),
		)
		.add_systems(Update, (user_input, actor_update_route))
		.add_systems(Update, actor_steering)
		.run();
}

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct SectorLabel(u32, u32);

/// Helper component attached to each sprite, allows for the visualisation to be updated, you wouldn't use this in a real simulation
#[derive(Component)]
struct FieldCellLabel(usize, usize);

/// Labels the actor to enable getting its [Transform] easily
#[derive(Component)]
struct Actor;

/// Attached to the actor as a record of where it is and where it wants to go, used to lookup the correct FlowField
#[allow(clippy::type_complexity)]
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Default, Component)]
struct Pathing {
	source_sector: Option<SectorID>,
	source_field_cell: Option<FieldCell>,
	target_position: Option<Vec2>,
	target_sector: Option<SectorID>,
	target_goal: Option<FieldCell>,
	portal_route: Option<Vec<(SectorID, FieldCell)>>,
	has_los: bool,
}

/// Spawn sprites to represent the world
fn setup_visualisation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	let map_length = 1920;
	let map_depth = 1920;
	let sector_resolution = 640;
	let actor_size = 16.0;
	let map_dimensions = MapDimensions::new(map_length, map_depth, sector_resolution, actor_size);
	let mut camera = Camera2dBundle::default();
	camera.projection.scale = 2.0;
	cmds.spawn(camera);
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
	let sector_cost_fields = SectorCostFields::from_ron(path, &map_dimensions);
	let fields = sector_cost_fields.get_baseline();
	// iterate over each sector field to place the sprites
	for (sector_id, field) in fields.iter() {
		// iterate over the dimensions of the field
		for (i, column) in field.get().iter().enumerate() {
			for (j, value) in column.iter().enumerate() {
				// grid origin is always in the top left
				let sector_offset = map_dimensions.get_sector_corner_xy(*sector_id);
				let x = sector_offset.x + 32.0 + (FIELD_SPRITE_DIMENSION * i as f32);
				let y = sector_offset.y - 32.0 - (FIELD_SPRITE_DIMENSION * j as f32);
				// add colliders to impassable cells
				if *value == 255 {
					cmds.spawn(SpriteBundle {
						sprite: Sprite {
							color: Color::BLACK,
							..default()
						},
						transform: Transform {
							translation: Vec3::new(x, y, 0.0),
							scale: Vec3::new(FIELD_SPRITE_DIMENSION, FIELD_SPRITE_DIMENSION, 1.0),
							..default()
						},
						..default()
					})
					.insert(FieldCellLabel(i, j))
					.insert(SectorLabel(sector_id.get_column(), sector_id.get_row()))
					.insert(Collider::cuboid(1.0, 1.0))
					.insert(RigidBody::Static)
					.insert(CollisionLayers::new([Layer::Terrain], [Layer::Actor]));
				} else {
					cmds.spawn(SpriteBundle {
						texture: asset_server.load(get_basic_icon(*value)),
						transform: Transform::from_xyz(x, y, 0.0),
						..default()
					})
					.insert(FieldCellLabel(i, j))
					.insert(SectorLabel(sector_id.get_column(), sector_id.get_row()));
				}
			}
		}
	}
}
/// Spawn navigation related entities
fn setup_navigation(mut cmds: Commands) {
	// create the entity handling the algorithm
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
	let map_length = 1920;
	let map_depth = 1920;
	let sector_resolution = 640;
	let actor_size = 16.0;
	cmds.spawn(FlowFieldTilesBundle::from_ron(
		map_length,
		map_depth,
		sector_resolution,
		actor_size,
		&path,
	));
	// create the controllable actors
	let actor_positions = [
		Vec3::new(928.0, 928.0, 1.0),
		Vec3::new(864.0, 928.0, 1.0),
		Vec3::new(800.0, 928.0, 1.0),
		Vec3::new(-928.0, 928.0, 1.0),
		Vec3::new(-864.0, 928.0, 1.0),
		Vec3::new(-800.0, 928.0, 1.0),
		Vec3::new(-928.0, -928.0, 1.0),
		Vec3::new(-864.0, -928.0, 1.0),
		Vec3::new(-928.0, -864.0, 1.0),
		Vec3::new(928.0, -928.0, 1.0),
		Vec3::new(864.0, -928.0, 1.0),
		Vec3::new(928.0, -864.0, 1.0),
	];
	for pos in actor_positions.iter() {
		cmds.spawn(SpriteBundle {
			sprite: Sprite {
				color: Color::Rgba {
					red: 230.0,
					green: 0.0,
					blue: 255.0,
					alpha: 1.0,
				},
				..default()
			},
			transform: Transform {
				translation: Vec3::new(pos.x, pos.y, pos.z),
				scale: Vec3::new(16.0, 16.0, 1.0),
				..default()
			},
			..default()
		})
		.insert(Actor)
		.insert(Pathing::default())
		.insert(RigidBody::Dynamic)
		.insert(Collider::cuboid(1.0, 1.0))
		.insert(CollisionLayers::new([Layer::Actor], [Layer::Terrain]));
	}
}

/// Handle generating a PathRequest via right click
fn user_input(
	mouse_button_input: Res<Input<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	dimensions_q: Query<&MapDimensions>,
	mut actor_q: Query<(&Transform, &mut Pathing), With<Actor>>,
	mut event: EventWriter<EventPathRequest>,
) {
	if mouse_button_input.just_released(MouseButton::Right) {
		// get 2d world positionn of cursor
		let (camera, camera_transform) = camera_q.single();
		let window = windows.single();
		if let Some(world_position) = window
			.cursor_position()
			.and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
			.map(|ray| ray.origin.truncate())
		{
			let map_dimensions = dimensions_q.get_single().unwrap();
			if let Some((target_sector_id, goal_id)) =
				map_dimensions.get_sector_and_field_id_from_xy(world_position)
			{
				for (tform, mut pathing) in actor_q.iter_mut() {
					let (source_sector_id, source_field_cell) = map_dimensions
						.get_sector_and_field_id_from_xy(tform.translation.truncate())
						.unwrap();
					event.send(EventPathRequest::new(
						source_sector_id,
						source_field_cell,
						target_sector_id,
						goal_id,
					));
					// update the actor pathing
					pathing.source_sector = Some(source_sector_id);
					pathing.source_field_cell = Some(source_field_cell);
					pathing.target_position = Some(world_position);
					pathing.target_sector = Some(target_sector_id);
					pathing.target_goal = Some(goal_id);
					pathing.portal_route = None;
				}
			} else {
				error!("Cursor out of bounds");
			}
		}
	}
}
/// There is a delay between the actor sending a path request and a route becoming available. This checks to see if the route is available and adds a copy to the actor
fn actor_update_route(mut actor_q: Query<&mut Pathing, With<Actor>>, route_q: Query<&RouteCache>) {
	for mut pathing in actor_q.iter_mut() {
		if pathing.target_goal.is_some() && pathing.portal_route.is_none() {
			let route_cache = route_q.get_single().unwrap();
			if let Some(route) = route_cache.get_route(
				pathing.source_sector.unwrap(),
				pathing.source_field_cell.unwrap(),
				pathing.target_sector.unwrap(),
				pathing.target_goal.unwrap(),
			) {
				pathing.portal_route = Some(route.clone());
			}
		}
	}
}
/// Actor speed
const SPEED: f32 = 50000.0;

/// If the actor has a destination set then try to retrieve the relevant
/// [FlowField] for its current position and move the actor
fn actor_steering(
	mut actor_q: Query<(&mut LinearVelocity, &mut Transform, &mut Pathing), With<Actor>>,
	flow_cache_q: Query<(&FlowFieldCache, &MapDimensions)>,
	time_step: Res<Time>,
) {
	let (flow_cache, map_dimensions) = flow_cache_q.get_single().unwrap();
	for (mut velocity, tform, mut pathing) in actor_q.iter_mut() {
		if pathing.target_goal.is_some() {
			// lookup the overarching route
			if let Some(route) = pathing.portal_route.as_mut() {
				// info!("Route: {:?}", route);
				// find the current actors postion in grid space
				let (curr_actor_sector, curr_actor_field_cell) = map_dimensions
					.get_sector_and_field_id_from_xy(tform.translation.truncate())
					.unwrap();
				// tirm the actor stored route as it makes progress
				// this ensures it doesn't use a previous goal from
				// a sector it has already been through when it needs
				// to pass through it again as part of a different part of the route
				if curr_actor_sector != route.first().unwrap().0 {
					route.remove(0);
				}
				// lookup the relevant sector-goal of this sector
				'routes: for (sector, goal) in route.iter() {
					if *sector == curr_actor_sector {
						// get the flow field
						if let Some(field) = flow_cache.get_field(*sector, *goal) {
							// based on actor field cell find the directional vector it should move in
							let cell_value = field.get_field_cell_value(curr_actor_field_cell);
							if has_line_of_sight(cell_value) {
								pathing.has_los = true;
								let dir =
									pathing.target_position.unwrap() - tform.translation.truncate();
								velocity.0 = dir.normalize() * SPEED * time_step.delta_seconds();
								break 'routes;
							}
							let dir = get_2d_direction_unit_vector_from_bits(cell_value);
							velocity.0 = dir * SPEED * time_step.delta_seconds();
						}
						break 'routes;
					}
				}
			}
		}
	}
}

/// Get asset path of sprite icons
fn get_basic_icon(value: u8) -> String {
	if value == 255 {
		String::from("ordinal_icons/impassable.png")
	} else if value == 1 {
		String::from("ordinal_icons/goal.png")
	} else {
		panic!("Require basic icon")
	}
}

/// Used in CollisionLayers so that actors don't collide with one another, only the terrain
#[allow(clippy::missing_docs_in_private_items)]
enum Layer {
	Actor,
	Terrain,
}

// weird bug when using #derive where it thinks the crate bevy_xpbd_3d is being used >(
impl PhysicsLayer for Layer {
	fn to_bits(&self) -> u32 {
		match self {
			Layer::Actor => 1,
			Layer::Terrain => 2,
		}
	}

	fn all_bits() -> u32 {
		0b11
	}
}

/// Create collider entities around the world
fn create_wall_colliders(mut cmds: Commands) {
	let top_location = Vec3::new(0.0, FIELD_SPRITE_DIMENSION * 15.0, 0.0);
	let top_scale = Vec3::new(
		FIELD_SPRITE_DIMENSION * 30.0,
		FIELD_SPRITE_DIMENSION / 2.0,
		1.0,
	);
	let bottom_location = Vec3::new(0.0, -FIELD_SPRITE_DIMENSION * 15.0, 0.0);
	let bottom_scale = Vec3::new(
		FIELD_SPRITE_DIMENSION * 30.0,
		FIELD_SPRITE_DIMENSION / 2.0,
		1.0,
	);
	let left_location = Vec3::new(-FIELD_SPRITE_DIMENSION * 15.0, 0.0, 0.0);
	let left_scale = Vec3::new(
		FIELD_SPRITE_DIMENSION / 2.0,
		FIELD_SPRITE_DIMENSION * 30.0,
		1.0,
	);
	let right_location = Vec3::new(FIELD_SPRITE_DIMENSION * 15.0, 0.0, 0.0);
	let right_scale = Vec3::new(
		FIELD_SPRITE_DIMENSION / 2.0,
		FIELD_SPRITE_DIMENSION * 30.0,
		1.0,
	);

	let walls = [
		(top_location, top_scale),
		(bottom_location, bottom_scale),
		(left_location, left_scale),
		(right_location, right_scale),
	];

	for (loc, scale) in walls.iter() {
		cmds.spawn((
			SpriteBundle {
				transform: Transform {
					translation: *loc,
					scale: *scale,
					..default()
				},
				sprite: Sprite {
					color: Color::BLACK,
					..default()
				},
				..default()
			},
			RigidBody::Static,
			Collider::cuboid(1.0, 1.0),
			CollisionLayers::new([Layer::Terrain], []),
		));
	}
}
