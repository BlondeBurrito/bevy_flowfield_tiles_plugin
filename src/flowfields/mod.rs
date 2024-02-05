//! Flowfields are a means of handling pathfinding for a crowd of actors.
//!
//! To generate a set of navigation `FlowFields` the game world is divided into Sectors indexed by `(column, row)` and each Sector has 3 layers of data: `[CostField, IntegrationField, Flowfield]`. Each layer aids the next in building out a path. A concept of `Portals` is used to connect Sectors together.
//!
//! ## Useful definitions
//!
//! * Sector - a slice of a game world composed of three 2D arrays called fields (`CostField`, `IntegrationField` and `FlowField`). A game world is effectively represented by a number of Sectors
//! * CostField - a 2D array describing how difficult it is to path through each cell of the array. It is always present in system memory
//! * Cost - how difficult/expensive it is to path somewhere, you could also call it <i>weight</i>, each cell of `CostField` has one of these
//! * Portal - a navigatable point which links one Sector to another to enable movement from one side of the world to another
//! * IntegrationField - a 2D array which uses the CostField to determine a cumulative cost of reaching the goal/endpoint (where you want to path to). This is an ephemeral field - it exists when required to calculate a `FlowField`
//! * FlowField - a 2D array built from the `IntegrationField` which decribes how an actor should move (flow) across the world
//! * FlowField Cache - a means of storing `FlowFields` allowing multiple actors to use and reuse them
//! * Ordinal - a direction based on traditional compass ordinals: N, NE, E, SE, S, SW, W, NW. Used for discovery of Sectors/field cells at various points within the algorithm
//! * Field cell - an element of a 2D array
//! * Goal - the target field cell an actor needs to path to
//! * Portal goal - a target point within a sector that allows an actor to transition to another sector, thus bringing it closer towards/to the goal
//!
//! ## Sector
//!
//! For a 3-dimensional world the `x-z` (`x-y` in 2d) plane defines the number of Sectors used to represent it with a constant called `SECTOR_RESOLUTION`, currently enforced at `10`. This means that a for a `30x30` world there would be `3x3` Sectors representing it. Each Sector has an associated unqiue ID taken as its position: `(column, row)`.
//!
//! ## CostField
//!
//! A `CostField` is an `MxN` 2D array of 8-bit values. The values indicate the `cost` of navigating through that cell of the field. A value of `1` is the default and indicates the easiest `cost`, and a value of `255` is a special value used to indicate that the field cell is impassable - this could be used to indicate a wall or obstacle. All other values from `2-254` represent increasing cost, for instance a slope or difficult terrain such as a marsh. The idea is that the pathfinding calculations will favour cells with a smaller value before any others.
//!
//!
//! ## Portals
//!
//! Each Sector has up to 4 boundaries with neighbouring Sectors (fewer when the sector is in a corner or along the edge of the game world). Each boundary can contain Portals which indicate a navigatable point from the current Sector to a neighbour. Portals serve a dual purpose, one of which is to provide responsiveness - `FlowFields` may take time to generate so when an actor needs to move a quick A* pathing query can produce an inital path route based on moving from one Portal to another and they can start moving in the general direction to the goal/target/endpoint. Once the `FlowFields` have been built the actor can switch to using them for granular navigation instead.
//!
//! ## Portal Graph
//!
//! For finding a path from one Sector to another at a Portal level all Sector Portals are recorded within a data strucutre known as `PortalGraph`. The Portals are stored as Nodes and Edges are created between them to represent traversable paths, it gets built in three stages:
//!
//! * For all Portals add a graph `node`
//! * For each sector create `edges` (pathable routes) to and from each Portal `node` - effectively create internal walkable routes of each sector
//! * Create `edges` across the Portal `node` on all sector boundaries (walkable route from one sector to another)
//!
//! This allows the graph to be queried with a `source` sector and a `target` sector and a list of Portals are returned which can be pathed. When a `CostField` is changed this triggers the regeneration of the sector Portals for the region that `CostField` resides in (and its neighbours to ensure homogenous boundaries) and the graph is updated with any new Portals `nodes` and the old ones are removed.
//!
//! ## IntegrationField
//!
//! An `IntegrationField` is an `MxN` 2D array of 16-bit values. It uses the `CostField` to produce a cumulative cost to reach the end goal/target. It's an ephemeral field, as in it gets built for a required sector and then consumed by the `FlowField` calculation.
//!
//! When a new route needs to be processed the field values are set to `u16::MAX` and the field cell containing the goal is set to `0`.
//!
//! A series of passes are performed from the goal as an expanding wavefront calculating the field values:
//!
//! * The valid ordinal neighbours of the goal are determined (North, East, South, West - when not against a sector/world boundary)
//! * For each ordinal field cell lookup their `CostField` value
//! * Add the `CostField` cost to the `IntegrationFields` cost of the current cell (at the beginning this is the goal int cost `0`)
//! * Propagate to the next neighbours, find their ordinals and repeat adding their cost value to to the current cells integration cost to produce their cumulative integration cost, and repeat until the entire field is done
//!
//! ## FlowField
//!
//! A `FlowField` is an `MxN` 2D array of 8-bit values built from a Sectors `IntegrationField`. The first 4 bits of the value correspond to one of eight ordinal movement directions an actor can take (plus a zero vector when impassable) and the second 4 bits correspond to flags which should be used by a character controller/steering pipeline to follow a path.
//!
//! ## Caching FlowFields
//!
//! To enable actors to reuse `FlowFields` (thus avoiding repeated calculations) a pair of caches are used to store pathing data:
//!
//! * Route Cache - when an actor requests to go somewhere a high-level route is generated from describing the overall series of sector-portals to traverse (`PortalGraph` A*). If a `FlowField` hasn't yet been calculated then an actor can use the `route_cache` as a fallback to gain a generalist direction they should start moving in. Once the `FlowFields` have been built they can swap over to using those more granular paths. Additionally changes to `CostFields` can change portal positions and the real best path, so `FlowFields` are regenerated for the relevant sectors that `CostFields` have modified and during the regeneration steps an actor can once again use the high-level route as the fallback
//!
//! * Field Cache - for every sector-to-portal part of a route a `FlowField` is built and stored in the cache. Actors can poll this cache to get the true flow direction to their goal. A Character Controller/Steering Pipeline is responsible for interpreting the values of the `FlowField` to produce movement - while this plugin includes a Steering Pipeline the reality is that every game has it's own quirks and desires for movement so you will most likely want to build your own Pipeline. The real point of this plugin is to encapulsate the data structures and logic to make a `FlowField` which an Actor can then read through it's own implementation.
//!

pub mod fields;
pub mod portal;
pub mod sectors;
pub mod utilities;

// #[rustfmt::skip]
// #[cfg(test)]
// mod tests {
// 	use super::*;
// }
