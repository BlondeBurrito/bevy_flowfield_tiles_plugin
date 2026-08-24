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
	// create each integration field
	// reverse iter so starting at goal and calculate fields towards source
	// NB: this means `ints` is in order of goal to source
	let mut ints = vec![];
	for step in path.iter().rev() {
		let sector = step.get_sector();
		let scaled_costfields = sector_cost_fields.get_scaled_costs();
		let scaled_costfield = scaled_costfields.get(sector).unwrap();

		let mut integrationfield = IntegrationField::init(scaled_costfield, step);
		integrationfield.build(scaled_costfield);
		ints.push(integrationfield);
	}
	// build each flowfield
	// reverse iter so starting at goal and calculate fields towards source
	// we don't need to flip the index (calling enumerate before rev)
	// because ints is in order of goal to source
	for (i, step) in path.iter().rev().enumerate() {
		if i == 0 {
			let mut flowfield = FlowField::new(step, &ints[i], None);
			flowfield.build(&ints[i]);
			generated.push((*step, flowfield));
		} else {
			let mut flowfield = FlowField::new(step, &ints[i], Some(&ints[i - 1]));
			flowfield.build(&ints[i]);
			generated.push((*step, flowfield));
		}
	}
}
