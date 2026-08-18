//! Helpers for getting sprite assets so some examples can visually show flow lines

use bevy_flowfield_tiles_plugin::v2::flowfields::{
	fields::flow_field::{get_ordinal_from_bits, has_line_of_sight, is_goal, is_wall},
	utilities::Ordinal,
};

/// Get asset path of sprite assets
pub fn get_basic_icon(value: u8) -> String {
	if value == 255 {
		String::from("ordinal_icons/impassable.png")
	} else if value == 1 {
		String::from("ordinal_icons/goal.png")
	} else {
		panic!("Require basic icon")
	}
}
/// Get asset path of ordinal icon
pub fn get_ord_icon(value: u8) -> String {
	if is_goal(value) {
		return String::from("ordinal_icons/goal.png");
	}
	//
	if has_line_of_sight(value) {
		return String::from("ordinal_icons/los.png");
	}
	//
	if is_wall(value) {
		return String::from("ordinal_icons/impassable.png");
	}
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
