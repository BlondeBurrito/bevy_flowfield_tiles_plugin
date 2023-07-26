//! Calculates the [FlowField]s from a set of [CostField]s and displays the cell values in a UI grid.
//!
//! For sectors which an actor does not need to traverse they are not generated or rendered
//!

use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy_flowfield_tiles_plugin::prelude::*;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_systems(Startup, (setup,))
		.run();
}
/// Init world
fn setup(mut cmds: Commands, asset_server: Res<AssetServer>) {
	// calculate the fields
	let map_dimensions = MapDimensions::new(30, 30, 10);
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
	let sector_cost_fields = SectorCostFields::from_ron(path);
	let mut sector_portals =
		SectorPortals::new(map_dimensions.get_length(), map_dimensions.get_depth(), map_dimensions.get_sector_resolution());
	// update default portals for cost fields
	for sector_id in sector_cost_fields.get().keys() {
		sector_portals.update_portals(
			*sector_id,
			&sector_cost_fields,
			&map_dimensions
		);
	}
	// generate the portal graph
	let portal_graph = PortalGraph::new(
		&sector_portals,
		&sector_cost_fields,
		&map_dimensions
	);
	//
	let source_sector = SectorID::new(2, 0);
	let source_field_cell = FieldCell::new(7, 3);
	let target_sector = SectorID::new(0, 2);
	let target_field_cell = FieldCell::new(0, 6);
	// path from actor to goal sectors
	let node_path = portal_graph
		.find_best_path(
			(source_sector, source_field_cell),
			(target_sector, target_field_cell),
			&sector_portals,
			&sector_cost_fields,
		)
		.unwrap();
	// convert to field cell and sector coords
	let mut path =
		portal_graph.convert_index_path_to_sector_portal_cells(node_path.1, &sector_portals);
	let mut path_based_on_portal_exits = Vec::new();
	// target sector and entry portal where we switch the entry portal cell to the goal
	let mut end = path.pop().unwrap();
	end.1 = target_field_cell;
	// sector and field of leaving starting sector if source sector and target sector are different
	// otherwise it was a single element path and we already removed it
	if !path.is_empty() {
		let start = path.remove(0);
		path_based_on_portal_exits.push(start);
	}
	// all other elements in the path are in pairs for entering and leaving sectors on the way to the goal
	for p in path.iter().skip(1).step_by(2) {
		path_based_on_portal_exits.push(*p);
	}
	path_based_on_portal_exits.push(end);
	path = path_based_on_portal_exits;
	// original order is from actor to goal, int fields need to be processed the other way around
	path.reverse();
	let mut sectors_expanded_goals = Vec::new();
	for (i, (sector_id, goal)) in path.iter().enumerate() {
		// first element is always the end target, don't bother with portal expansion
		if i == 0 {
			sectors_expanded_goals.push((*sector_id, vec![*goal]));
		} else {
			// portals represent the boundary to another sector, a portal can be spread over
			// multple field cells, expand the portal to provide multiple goal
			// targets for moving to another sector
			let neighbour_sector_id = path[i - 1].0;
			let g = sector_portals
				.get()
				.get(sector_id)
				.unwrap()
				.expand_portal_into_goals(
					&sector_cost_fields,
					sector_id,
					goal,
					&neighbour_sector_id,
					&map_dimensions
				);
			sectors_expanded_goals.push((*sector_id, g));
		}
	}
	// prep int fields
	let mut sector_int_fields = Vec::new();
	for (sector_id, goals) in sectors_expanded_goals.iter() {
		let mut int_field = IntegrationField::new(goals);
		let cost_field = sector_cost_fields.get().get(sector_id).unwrap();
		int_field.calculate_field(goals, cost_field);
		sector_int_fields.push((*sector_id, goals.clone(), int_field));
	}
	// create flow fields
	let mut sector_flow_fields = BTreeMap::new();
	for (i, (sector_id, goals, int_field)) in sector_int_fields.iter().enumerate() {
		let mut flow_field = FlowField::default();
		if *sector_id == target_sector {
			flow_field.calculate(goals, None, int_field);
			sector_flow_fields.insert(*sector_id, flow_field);
		} else if let Some(dir_prev_sector) =
			Ordinal::sector_to_sector_direction(sector_int_fields[i - 1].0, *sector_id)
		{
			let prev_int_field = &sector_int_fields[i - 1].2;
			flow_field.calculate(goals, Some((dir_prev_sector, prev_int_field)), int_field);
			sector_flow_fields.insert(*sector_id, flow_field);
		};
	}
	// create a UI grid
	cmds.spawn(Camera2dBundle::default());
	cmds.spawn(NodeBundle {
		// background canvas
		style: Style {
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			flex_direction: FlexDirection::Column,
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			..Default::default()
		},
		background_color: BackgroundColor(Color::NONE),
		..Default::default()
	})
	.with_children(|p| {
		// a centred box to contain the fields
		p.spawn(NodeBundle {
			style: Style {
				width: Val::Px(1000.0),
				height: Val::Px(1000.0),
				flex_direction: FlexDirection::Column,
				flex_wrap: FlexWrap::Wrap,
				flex_shrink: 0.0,
				..Default::default()
			},
			background_color: BackgroundColor(Color::WHITE),
			..Default::default()
		})
		.with_children(|p| {
			// create an area for each sector int field
			for i in 0..map_dimensions.get_length() / 10 {
				for j in 0..map_dimensions.get_depth() / 10 {
					// bounding node of a sector
					p.spawn(NodeBundle {
						style: Style {
							width: Val::Percent(100.0 / (map_dimensions.get_length() / 10) as f32),
							height: Val::Percent(100.0 / (map_dimensions.get_depth() / 10) as f32),
							flex_direction: FlexDirection::Column,
							flex_wrap: FlexWrap::Wrap,
							flex_shrink: 0.0,
							..Default::default()
						},
						..Default::default()
					})
					.with_children(|p| {
						// the array area of the sector
						let flow_field = sector_flow_fields.get(&SectorID::new(i, j));
						if let Some(field) = flow_field {
							// create each column from the field
							for array in field.get_field().iter() {
								p.spawn(NodeBundle {
									style: Style {
										width: Val::Percent(10.0),
										height: Val::Percent(100.0),
										flex_direction: FlexDirection::Column,
										..Default::default()
									},
									..Default::default()
								})
								.with_children(|p| {
									// create each row value of the column
									for value in array.iter() {
										p.spawn((
											NodeBundle {
												style: Style {
													width: Val::Percent(100.0),
													height: Val::Percent(10.0),
													justify_content: JustifyContent::Center,
													align_items: AlignItems::Center,
													..Default::default()
												},
												background_color: BackgroundColor(Color::WHITE),
												..Default::default()
											},
											UiImage::new(asset_server.load(get_ord_icon(*value))),
										));
									}
								});
							}
						}
					});
				}
			}
		});
	});
}
/// Get the asset path of ordinal icons
fn get_ord_icon(value: u8) -> String {
	// temp
	if value == 64 {
		return String::from("ordinal_icons/goal.png");
	}
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
