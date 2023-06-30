//! Calculates the [IntegrationField]s from a set of [CostField]s and displays the cell values in a UI grid
//!

use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy_flowfield_tiles_plugin::flowfields::{
	cost_field::CostField, integration_field::IntegrationField, sectors::{SectorCostFields, SectorPortals}, MapDimensions, portal::portal_graph::PortalGraph,
};

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_systems(Startup, (setup,))
		.run();
}

fn setup(mut cmds: Commands, asset_server: Res<AssetServer>) {
	// calculate the fields
	let map_dimensions = MapDimensions::new(30, 30);
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
	let sector_cost_fields = SectorCostFields::from_file(path);
	let mut sector_portals = SectorPortals::new(map_dimensions.get_column(), map_dimensions.get_row());
	// update default portals for cost fields
	for (sector_id, _v) in sector_cost_fields.get() {
		sector_portals.update_portals(*sector_id, &sector_cost_fields, map_dimensions.get_column(), map_dimensions.get_row());
	}
	// generate the portal graph
	let portal_graph = PortalGraph::new(&sector_portals, map_dimensions.get_column(), map_dimensions.get_row());
	// portal_graph.build_graph_nodes(&sector_portals);
	// portal_graph.build_edges_within_each_sector(&sector_portals);
	// portal_graph.build_edges_between_sectors(&sector_portals, map_x_dimension, map_z_dimension);
	// update the graph to reflect what cost fields were loaded from disk
	// for (sector_id, _v) in sector_cost_fields.get() {
	// 	portal_graph.update_graph(*sector_id, &sector_portals, map_dimensions.get_column(), map_dimensions.get_row());
	// }
	//
	let source_sector = (0, 0);
	let source_grid_cell = (7, 3);
	let target_sector = (0, 2);
	let target_grid_cell = (0, 6);
	// keys are the sectors enroute to target sector (including target sector)
	// values are portal positions, final element of each value is the destination portal/goal of that sector
	let path = portal_graph.find_path_of_sector_grid_indices(source_sector, target_sector, &sector_portals).unwrap();
	// prep int fields
	let mut sector_int_fields = BTreeMap::new();
	for (sector_id, goals) in path.iter() {
		let mut int_field = IntegrationField::new(*goals.last().unwrap());
		let cost_field = sector_cost_fields.get().get(sector_id).unwrap();
		int_field.calculate_field(*goals.last().unwrap(), cost_field);
		sector_int_fields.insert(sector_id, int_field);
	}



	// create a UI grid
	cmds.spawn(Camera2dBundle::default());
	cmds.spawn(NodeBundle {
		// background canvas
		style: Style {
			size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
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
				size: Size::new(Val::Px(1000.0), Val::Px(1000.0)),
				flex_direction: FlexDirection::Column,
				flex_shrink: 0.0,
				..Default::default()
			},
			background_color: BackgroundColor(Color::WHITE),
			..Default::default()
		})
		.with_children(|p| {
			// create an area for each sector int field
			for i in 0..map_dimensions.get_column() / 10 {
				for j in 0..map_dimensions.get_row() / 10 {
					// bounding node of a sector
					p.spawn(NodeBundle{
						style: Style {
							size: Size::new(Val::Percent(100.0 / ((map_dimensions.get_column() / 10)) as f32), Val::Percent(100.0 / ((map_dimensions.get_row() / 10)) as f32)),
							flex_direction: FlexDirection::Column,
							flex_wrap: FlexWrap::Wrap,
							flex_shrink: 0.0,
							..Default::default()
						},
						..Default::default()
					}).with_children(|p| {
						// the array area of the sector
						let int_field = sector_int_fields.get(&(i, j));
						match int_field {
							Some(field) => {
								// create each column from the field
								for array in field.get_field().iter() {
									p.spawn(NodeBundle {
										style: Style {
											size: Size::new(Val::Percent(10.0), Val::Percent(100.0)),
											flex_direction: FlexDirection::Column,
											..Default::default()
										},
										..Default::default()
									})
									.with_children(|p| {
										// create each row value of the column
										for value in array.iter() {
											p.spawn(NodeBundle {
												style: Style {
													size: Size::new(Val::Percent(100.0), Val::Percent(10.0)),
													justify_content: JustifyContent::Center,
													align_items: AlignItems::Center,
													..Default::default()
												},
												..Default::default()
											})
											.with_children(|p| {
												p.spawn(TextBundle::from_section(
													value.to_string(),
													TextStyle {
														font: asset_server.load("fonts/FiraSans-Bold.ttf"),
														font_size: 10.0,
														color: Color::BLACK,
													},
												));
											});
										}
									});
								}
							},
							None => {},
						}
					});
				}
			}










			// // create each column from the field
			// for array in int_field.get_field().iter() {
			// 	p.spawn(NodeBundle {
			// 		style: Style {
			// 			size: Size::new(Val::Percent(10.0), Val::Percent(100.0)),
			// 			flex_direction: FlexDirection::Column,
			// 			..Default::default()
			// 		},
			// 		..Default::default()
			// 	})
			// 	.with_children(|p| {
			// 		// create each row value of the column
			// 		for value in array.iter() {
			// 			p.spawn(NodeBundle {
			// 				style: Style {
			// 					size: Size::new(Val::Percent(100.0), Val::Percent(10.0)),
			// 					justify_content: JustifyContent::Center,
			// 					align_items: AlignItems::Center,
			// 					..Default::default()
			// 				},
			// 				..Default::default()
			// 			})
			// 			.with_children(|p| {
			// 				p.spawn(TextBundle::from_section(
			// 					value.to_string(),
			// 					TextStyle {
			// 						font: asset_server.load("fonts/FiraMono-Medium.ttf"),
			// 						font_size: 15.0,
			// 						color: Color::BLACK,
			// 					},
			// 				));
			// 			});
			// 		}
			// 	});
			// }
		});
	});
}
