//! Helpers for getting sprite assets so some examples can visually show flow lines

use bevy_flowfield_tiles_plugin::prelude::*;

/// Get asset path of sprite assets
#[allow(dead_code)]
pub fn get_basic_icon(value: u8) -> String {
	if value == 255 {
		String::from("compass_dir_icons/impassable.png")
	} else if value == 1 {
		String::from("compass_dir_icons/goal.png")
	} else {
		panic!("Require basic icon")
	}
}
/// Get asset path of compass dir icon
#[allow(dead_code)]
pub fn get_compass_dir_icon(value: u8) -> String {
	if is_goal(value) {
		return String::from("compass_dir_icons/goal.png");
	}
	//
	if has_line_of_sight(value) {
		return String::from("compass_dir_icons/los.png");
	}
	//
	if is_wall(value) {
		return String::from("compass_dir_icons/impassable.png");
	}
	let compass_dir = get_compass_dir_from_bits(value);
	match compass_dir {
		CompassDir::North => String::from("compass_dir_icons/north.png"),
		CompassDir::East => String::from("compass_dir_icons/east.png"),
		CompassDir::South => String::from("compass_dir_icons/south.png"),
		CompassDir::West => String::from("compass_dir_icons/west.png"),
		CompassDir::NorthEast => String::from("compass_dir_icons/north_east.png"),
		CompassDir::SouthEast => String::from("compass_dir_icons/south_east.png"),
		CompassDir::SouthWest => String::from("compass_dir_icons/south_west.png"),
		CompassDir::NorthWest => String::from("compass_dir_icons/north_west.png"),
		CompassDir::Zero => String::from("compass_dir_icons/impassable.png"),
	}
}
