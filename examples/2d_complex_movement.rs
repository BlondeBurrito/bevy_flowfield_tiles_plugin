//! Generates a 30x30 world where an actor can be told to move through a narrow snake-like path
//!
//TODO visualisation creates impassable blocks when route goes back on self
use std::{collections::HashMap, time::Duration};

use bevy::{
	prelude::*,
	sprite::collide_aabb::{collide, Collision},
	window::PrimaryWindow,
};
use bevy_flowfield_tiles_plugin::prelude::*;

/// Timestep of actor movement system
const ACTOR_TIMESTEP: f32 = 1.0 / 60.0;
/// Dimension of square sprites making up the world
const FIELD_SPRITE_DIMENSION: f32 = 64.0;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f32(
			ACTOR_TIMESTEP,
		)))
		.add_plugins(FlowFieldTilesPlugin)
		.add_systems(
			Startup,
			(setup_visualisation, setup_navigation, create_wall_colliders),
		)
		.add_systems(Update, (user_input, actor_update_route))
		.add_systems(Update, (update_sprite_visuals_based_on_actor,))
		.add_systems(
			FixedUpdate,
			(actor_steering, collision_detection, apply_velocity).chain(),
		)
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
	current_direction: Option<Vec2>,
	/// Helps to steer the actor around corners when it is very close to an impassable field cell and reduces the likihood on tunneling
	previous_direction: Option<Vec2>,
	has_los: bool,
}

/// Dir and magnitude of actor movement
#[derive(Component, Default)]
struct Velocity(Vec2);

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
	let dir = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/csv/vis_portals/";
	let sector_cost_fields = SectorCostFields::from_csv_dir(&map_dimensions, dir);
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
						texture: asset_server.load(get_basic_icon(*value)),
						transform: Transform::from_xyz(x, y, 0.0),
						..default()
					})
					.insert(FieldCellLabel(i, j))
					.insert(SectorLabel(sector_id.get_column(), sector_id.get_row()))
					.with_children(|p| {
						// required to have a collider sized correctly
						p.spawn(SpatialBundle {
							transform: Transform::from_scale(Vec3::new(
								FIELD_SPRITE_DIMENSION,
								FIELD_SPRITE_DIMENSION,
								1.0,
							)),
							..default()
						})
						.insert(Collider);
					});
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
fn setup_navigation(mut cmds: Commands, asset_server: Res<AssetServer>) {
	// create the entity handling the algorithm
	let dir = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/csv/vis_portals/";
	let map_length = 1920;
	let map_depth = 1920;
	let sector_resolution = 640;
	let actor_size = 16.0;
	cmds.spawn(FlowFieldTilesBundle::from_csv(
		map_length,
		map_depth,
		sector_resolution,
		actor_size,
		&dir,
	));
	// create the controllable actor in the top right corner
	cmds.spawn(SpriteBundle {
		texture: asset_server.load("2d/2d_actor_sprite.png"),
		transform: Transform::from_xyz(-928.0, -928.0, 1.0),
		..default()
	})
	.insert(Actor)
	.insert(Velocity::default())
	.insert(Pathing::default())
	.with_children(|p| {
		p.spawn(SpatialBundle {
			transform: Transform::from_scale(Vec3::new(16.0, 16.0, 1.0)),
			..default()
		});
	});
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
				let (tform, mut pathing) = actor_q.get_single_mut().unwrap();
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
			} else {
				error!("Cursor out of bounds");
			}
		}
	}
}
/// There is a delay between the actor sending a path request and a route becoming available. This checks to see if the route is available and adds a copy to the actor
fn actor_update_route(mut actor_q: Query<&mut Pathing, With<Actor>>, route_q: Query<&RouteCache>) {
	let mut pathing = actor_q.get_single_mut().unwrap();
	if pathing.target_goal.is_some() && pathing.portal_route.is_none() {
		let route_cache = route_q.get_single().unwrap();
		if let Some(route) = route_cache.get_route(
			pathing.source_sector.unwrap(),
			pathing.target_sector.unwrap(),
			pathing.target_goal.unwrap(),
		) {
			pathing.portal_route = Some(route.clone());
		}
	}
}
/// Actor speed measured in pixels per fixed tick
const SPEED: f32 = 250.0;

/// If the actor has a destination set then try to retrieve the relevant
/// [FlowField] for its current position and move the actor
fn actor_steering(
	mut actor_q: Query<(&mut Velocity, &mut Transform, &mut Pathing), With<Actor>>,
	flow_cache_q: Query<(&FlowFieldCache, &MapDimensions)>,
	time_step: Res<Time>,
) {
	let (mut velocity, tform, mut pathing) = actor_q.get_single_mut().unwrap();
	let (flow_cache, map_dimensions) = flow_cache_q.get_single().unwrap();

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
						if pathing.current_direction.is_none() {
							pathing.current_direction = Some(dir);
						} else if pathing.current_direction.unwrap() != dir {
							pathing.previous_direction = pathing.current_direction;
							pathing.current_direction = Some(dir);
						}
						velocity.0 = dir * SPEED * time_step.delta_seconds();
					}
					break 'routes;
				}
			}
		}
	}
}

/// Move the actor
fn apply_velocity(mut actor_q: Query<(&Velocity, &mut Transform), With<Actor>>) {
	for (velocity, mut tform) in actor_q.iter_mut() {
		tform.translation += velocity.0.extend(0.0);
	}
}
/// Get asset path to sprite icons
fn get_basic_icon(value: u8) -> String {
	if value == 255 {
		String::from("ordinal_icons/impassable.png")
	} else if value == 1 {
		String::from("ordinal_icons/goal.png")
	} else {
		panic!("Require basic icon")
	}
}

/// Whenever the actor has a path assigned attempt to get the current flowfield and update all the map sprites to visualise the directions of flow
fn update_sprite_visuals_based_on_actor(
	actor_q: Query<&Pathing, With<Actor>>,
	flowfield_q: Query<&FlowFieldCache>,
	costfield_q: Query<&SectorCostFields>,
	mut field_cell_q: Query<(&mut Handle<Image>, &FieldCellLabel, &SectorLabel)>,
	asset_server: Res<AssetServer>,
) {
	let f_cache = flowfield_q.get_single().unwrap();
	let sc_cache = costfield_q.get_single().unwrap();
	let pathing = actor_q.get_single().unwrap();
	if let Some(route) = &pathing.portal_route {
		let mut route_map: HashMap<SectorID, FieldCell> = HashMap::new();
		for (s, g) in route.iter() {
			route_map.insert(*s, *g);
		}
		for (mut handle, field_cell_label, sector_label) in field_cell_q.iter_mut() {
			// look for the value in the route_map if it's part of the flow, otherwise use the cost field
			if route_map.contains_key(&SectorID::new(sector_label.0, sector_label.1)) {
				let goal = route_map
					.get(&SectorID::new(sector_label.0, sector_label.1))
					.unwrap();
				if let Some(flowfield) =
					f_cache.get_field(SectorID::new(sector_label.0, sector_label.1), *goal)
				{
					let flow_value = flowfield.get_field_cell_value(FieldCell::new(
						field_cell_label.0,
						field_cell_label.1,
					));
					let icon = get_ord_icon(flow_value);
					let new_handle: Handle<Image> = asset_server.load(icon);
					*handle = new_handle;
				}
			} else {
				let value = sc_cache
					.get_baseline()
					.get(&SectorID::new(sector_label.0, sector_label.1))
					.unwrap()
					.get_field_cell_value(FieldCell::new(field_cell_label.0, field_cell_label.1));
				let icon = get_basic_icon(value);
				let new_handle: Handle<Image> = asset_server.load(icon);
				*handle = new_handle;
			}
		}
	}
}
/// Get the asset path to ordinal icons
fn get_ord_icon(value: u8) -> String {
	if is_goal(value) {
		String::from("ordinal_icons/goal.png")
	} else if has_line_of_sight(value) {
		String::from("ordinal_icons/los.png")
	} else {
		//
		let ordinal = get_ordinal_from_bits(value);
		match ordinal {
			Ordinal::North => String::from("ordinal_icons/north.png"),
			Ordinal::East => String::from("ordinal_icons/east.png"),
			Ordinal::South => String::from("ordinal_icons/south.png"),
			Ordinal::West => String::from("ordinal_icons/west.png"),
			Ordinal::NorthEast => String::from("ordinal_icons/north_east.png"),
			Ordinal::SouthEast => String::from("ordinal_icons/south_east.png"),
			Ordinal::SouthWest => String::from("ordinal_icons/south_west.png"),
			Ordinal::NorthWest => String::from("ordinal_icons/north_west.png"),
			Ordinal::Zero => String::from("ordinal_icons/impassable.png"),
		}
	}
}

/// Added to entities that should block actors
#[derive(Component)]
struct Collider;

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
			Collider,
		));
	}
}

/// Rebound actors when they begin to overlap an impassable area
fn collision_detection(
	mut actor_q: Query<(&mut Velocity, &Transform, &Children, &Pathing), With<Actor>>,
	actor_child_q: Query<&Transform>,
	static_colliders: Query<(&Parent, &Transform), With<Collider>>,
	parent_colliders: Query<&Transform>,
	time_step: Res<Time>,
) {
	for (mut velocity, actor_tform, children, pathing) in actor_q.iter_mut() {
		for (parent, child_collider_tform) in static_colliders.iter() {
			let parent_collider_tform = parent_colliders.get(parent.get()).unwrap();
			for &child in children {
				let tform = actor_child_q.get(child).unwrap();
				let collision = collide(
					actor_tform.translation,
					tform.scale.truncate(),
					parent_collider_tform.translation,
					child_collider_tform.scale.truncate(),
				);
				if let Some(collision) = collision {
					// direct the actor away from the collider
					match collision {
						Collision::Left => {
							velocity.0.x *= -1.0;
							if let Some(dir) = pathing.previous_direction {
								velocity.0.y = dir.y * SPEED * time_step.delta_seconds() * 2.0;
							}
						}
						Collision::Right => {
							velocity.0.x *= -1.0;
							if let Some(dir) = pathing.previous_direction {
								velocity.0.y = dir.y * SPEED * time_step.delta_seconds() * 2.0;
							}
						}
						Collision::Top => {
							velocity.0.y *= -1.0;
							if let Some(dir) = pathing.previous_direction {
								velocity.0.x = dir.x * SPEED * time_step.delta_seconds() * 2.0;
							}
						}
						Collision::Bottom => {
							velocity.0.y *= -1.0;
							if let Some(dir) = pathing.previous_direction {
								velocity.0.x = dir.x * SPEED * time_step.delta_seconds() * 2.0;
							}
						}
						Collision::Inside => {
							velocity.0 *= -1.0;
						}
					}
				}
			}
		}
	}
}
