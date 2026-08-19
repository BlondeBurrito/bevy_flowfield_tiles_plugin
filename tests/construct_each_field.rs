//! Try building each field
//!

use bevy_flowfield_tiles_plugin::prelude::*;

#[test]
/// Try building a set of FlowFields
fn field_on_field() {
	let path = env!("CARGO_MANIFEST_DIR").to_string() + "/assets/sector_costfields.ron";
	let dimensions = Dimensions::new((0.0, 0.0), (30.0, 30.0), 1.0, 0.5);
	let sector_cost_fields = SectorCostFields::from_ron(path, &dimensions);
	let portal = Portals::new(&sector_cost_fields);

	let source_sector = SectorID::new(2, 0);
	let source_cell = FieldCell::new(9, 0);
	let goal_sector = SectorID::new(0, 2);
	let goal_cell = FieldCell::new(1, 8);
	let path = portal
		.find_path(
			&source_sector,
			&source_cell,
			&goal_sector,
			&goal_cell,
			&sector_cost_fields,
		)
		.unwrap();

	let mut generated = vec![];
	for step in path.iter() {
		let sector = step.get_sector();
		let scaled_costfields = sector_cost_fields.get_scaled_costs();
		let scaled_costfield = scaled_costfields.get(sector).unwrap();

		let mut integrationfield = IntegrationField::init(scaled_costfield, step);
		integrationfield.build(scaled_costfield);

		let mut flowfield = FlowField::new(step, &integrationfield);
		flowfield.build(&integrationfield);

		generated.push((*step, flowfield));
	}
}
