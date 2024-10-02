//! Calculates the [IntegrationField]s from a set of [CostField]s and displays the cell values in a UI grid.
//!
//! For sectors which an actor does not need to traverse they are not generated or rendered
//!

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
	let map_dimensions = MapDimensions::new(30, 30, 10, 1.0);
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_cost_fields.ron";
	let sector_cost_fields = SectorCostFields::from_ron(path, &map_dimensions);
	let mut sector_portals = SectorPortals::new(
		map_dimensions.get_length(),
		map_dimensions.get_depth(),
		map_dimensions.get_sector_resolution(),
	);
	// update default portals for cost fields
	for sector_id in sector_cost_fields.get_scaled().keys() {
		sector_portals.update_portals(*sector_id, &sector_cost_fields, &map_dimensions);
	}
	// generate the portal graph
	let portal_graph = PortalGraph::new(&sector_portals, &sector_cost_fields, &map_dimensions);
	println!("Graph: {:?}", portal_graph);
	//
	let source_sector = SectorID::new(2, 0);
	let source_field_cell = FieldCell::new(7, 3);
	let target_sector = SectorID::new(0, 2);
	let target_field_cell = FieldCell::new(0, 6);
	// path from actor to goal sectors
	let mut path = portal_graph
		.find_best_path(
			(source_sector, source_field_cell),
			(target_sector, target_field_cell),
			&sector_portals,
			&sector_cost_fields,
		)
		.unwrap();
	info!("path done {:?}", path);
	filter_path(&mut path, target_field_cell);
	path.reverse();
	let route = Route::new(path);
	let mut int_builder = IntegrationBuilder::new(route, &sector_cost_fields);
	int_builder.expand_field_portals(&sector_portals, &sector_cost_fields, &map_dimensions);
	int_builder.calculate_los();
	int_builder.build_integrated_cost(&sector_cost_fields);

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
						for (sector, _goals, int_field) in
							int_builder.get_integration_fields().iter()
						{
							if sector.get() == (i, j) {
								for column in int_field.get().iter() {
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
										for row_cost in column.iter() {
											p.spawn(NodeBundle {
												style: Style {
													width: Val::Percent(100.0),
													height: Val::Percent(10.0),
													justify_content: JustifyContent::Center,
													align_items: AlignItems::Center,
													..Default::default()
												},
												background_color: BackgroundColor(get_colour(
													*row_cost,
												)),
												..Default::default()
											})
											.with_children(|p| {
												p.spawn(TextBundle::from_section(
													(row_cost & INT_FILTER_BITS_COST).to_string(),
													TextStyle {
														font: asset_server
															.load("fonts/FiraSans-Bold.ttf"),
														font_size: 10.0,
														color: Color::BLACK,
													},
												));
											});
										}
									});
								}
							}
						}
					});
				}
			}
		});
	});
}
/// Get the colour of a UI node
fn get_colour(value: u32) -> Color {
	if value & INT_BITS_LOS == INT_BITS_LOS {
		return Color::WHITE;
	}
	let cost = value & INT_FILTER_BITS_COST;
	match cost {
		0 => Color::WHITE,
		1 => Color::srgb(1.0, 0.95, 0.68),
		65535 => Color::srgb(0.5, 0.5, 0.5),
		_ => Color::srgb(
			1.0,
			0.95 * 0.9_f32.powf(cost as f32),
			0.68 * 0.9_f32.powf(cost as f32),
		),
	}
}
