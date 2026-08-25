[![crates.io](https://img.shields.io/crates/v/bevy_flowfield_tiles_plugin)](https://crates.io/crates/bevy_flowfield_tiles_plugin)
[![docs.rs](https://docs.rs/bevy_flowfield_tiles_plugin/badge.svg)](https://docs.rs/bevy_flowfield_tiles_plugin)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/blondeburrito/bevy_flowfield_tiles_plugin#license)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/blondeburrito/bevy_flowfield_tiles_plugin/ci.yml)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/blondeburrito/bevy_flowfield_tiles_plugin/code-cov.yml?label=CodeCov>85%)

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/emblem.png" alt="e" width="300"/>

# bevy_flowfield_tiles_plugin

Inspired by the work of [Elijah Emerson](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter23_Crowd_Pathfinding_and_Steering_Using_Flow_Field_Tiles.pdf) and with inspiration from [leifnode](https://leifnode.com/2013/12/flow-field-pathfinding/) and [jdxdev](https://www.jdxdev.com/blog/2020/05/03/flowfields/) this is an attempt to implement the data structures and logic required to generate a Flowfield representation of a world which can be used to path find movable actors.

| bevy | bevy_flowfield_tiles_plugin |
|------|-----------------------------|
| 0.19 | 0.15 |
| 0.17 | 0.14 |
| 0.16 | 0.13 |
| 0.15 | 0.12 |
| 0.14 |  0.10 - 0.11  |
| 0.13 |  0.7 - 0.9  |
| 0.12 |  0.5 - 0.6  |
| 0.11 |  0.1 - 0.4  |

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/continuous_resized.gif" alt="crgif" width="300"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/2d_with_steering_cropped.gif" alt="sgif" width="350"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/3d_actor_movement_cropped.gif" alt="3sgif" width="400"/>

# Table of Contents

1. [Intro](#intro)
1. [Useful Definitions](#useful-definitions)
1. [Design/Process](#designprocess)
1. [Usage](#usage)
1. [Features](#features)
1. [Performance](#performance)
1. [License](#license)

## Intro

Pathfinding in games can take different forms and those forms have certain benefits aligned with the type of game they are being applied to. Generally people run across:

* Way-point Graph - points in space linked together, very strict structure, an actor will move from one way-point to another. Great for games played on a small grid where movement needs to be restricted to precise lines, will be cumbersome when multiple actors are sharing a path - particularly when actors have some kind of collision system in place
* NavMesh - a walkable surface generated from the topology of meshes in a game world defining a valid area of movement. It allows for a range of dynamic movement within the confines of the mesh and is a natural evolution of the Way-point Graph
* FlowField Tiles - a means of handling crowd and flocking behaviour by generating a flow field (vector field) describing how an actor flows across a world. A large number of actors can flow in unison to an endpoint while sharing the same pathing data structure - saving compute resources and time

For larger and larger environments with an increasing number of pathing actors it may be beneficial to adopt a FlowField based approach due to the data sharing and formation/group like movement it promotes. FlowField Tiles are complex, it's effectively akin to fluid mechanics, so this is an attempt to bring an agnostic implementation to the [Bevy](https://github.com/bevyengine/bevy/tree/main) game engine.

## Useful Definitions

* Sector - a slice of a game world composed of three arrays called fields (`CostField`, `IntegrationField` and `FlowField`). A game world is effectively represented by a number of Sectors
* CostField - an array describing how difficult it is to path through a region. Visually you can picture this as a grid of squares.
* Cost - how difficult/expensive it is to path somewhere, you could also call it <i>weight</i>, each cell of `CostField` has one of these
* Portal - a navigable point which links one Sector to another which enables movement from one side of the world to another
* IntegrationField - an array which uses the CostField to determine a cumulative cost of reaching the goal/endpoint (where you want to path to). This is an ephemeral field - it exists when required to calculate a `FlowField`
* FlowField - an array built from the `IntegrationField` which describes how an actor should move (flow) across the world
* FlowField Cache - a means of storing `FlowFields` allowing multiple actors to use and reuse them
* CompassDir - a direction based on traditional compass cardinals and ordinals: N, NE, E, SE, S, SW, W, NW. Used for discovery of Sectors/field cells at various points within the algorithm
* Field cell - an element of a 2D array
* Goal - the target field cell an actor needs to path to
* Portal goal - a target point within a sector that allows an actor to transition to another sector, thus bringing it closer towards/to the goal

# Design/Process

To generate a set of navigation `FlowFields` the game world is divided into a grid of Sectors indexed by `(column, row)` and each Sector has 3 layers of data: `[CostField, IntegrationField, Flowfield]`. Each layer aids the next in building out a path. A concept of `Portals` is used to connect Sectors together.

Topologically Sectors are arranged with a top-left convention, with 0..n columns reading left to right and 0..n rows reading top to bottom. This means that Sector (0, 0) lies in negative-x positive-y space for 2d usage, with increasing Sector columns being found along the x-axis in a positive direction and increasing Sector rows being found along the y-axis heading in a negative direction; for 3d Sector (0, 0) lies in negative-x and negative-z space, with increasing columns heading in a positive-x direction and increasing rows heading in a positive-z direction.

## Sector

<details>
<summary>Click to expand!</summary>

A Sector represents a `10x10` region of space and each has a unique ID based on its `(column, row)` position in space. Effectively a game world is a grid of Sectors.

For a 3-dimensional world the `x-z` (`x-y` in 2d) plane defines the number of Sectors used to represent it based on a `size` parameter and a dimension called `world_unit_size`. 

The `size` of a world must be a perfect factor of `world_unit_size * 10`. Examples:

- World size `(30, 30)`, unit size `1` (like a metre). This corresponds to `3x3` sectors.
- World size `(1920, 1920)` (e.g pixels), unit size `64` (like a sprite). This corresponds to `3x3` sectors.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/sectors.png" alt="sectors" width="250"/>

</details>

## CostField

<details>
<summary>Click to expand!</summary>

A `CostField` is an array of 8-bit values, by default this array is always length 100. Visually the field is like a grid and each array element can be accessed via the convention of a `FieldCell`, which identifies each cell based on a `(column, row)` convention. The values indicate the `cost` of navigating through that cell of the field. A value of `1` is the default and indicates the easiest `cost`, and a value of `255` is a special value used to indicate that the field cell is impassable - this could be used to indicate a wall or obstacle. All other values from `2-254` represent increasing cost, for instance a slope or difficult terrain such as a marsh. The idea is that the pathfinding calculations will favour cells with a smaller value before any others.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/cost_field.png" alt="cf" width="370"/>

At runtime the `CostField` is generated for each Sector with the default value. With the crate feature `ron` it is possible to load the fields from disk instead, or with the feature `heightmap` a greyscale png/jpeg can be used to seed the fields. See the [Usage](#usage) section below for details on updating the `CostFields` during an initial pass (i.e when loading a level) and tweaking it during gameplay for a world which dynamically evolves with obstacles (flipping a cell to to a higher cost or an impassable `255` when something like a wall is placed or the ground splits into a fissure).

This array is used to generate the `IntegrationField` when requesting a navigable path.

</details>

## Portals

<details>
<summary>Click to expand!</summary>

Each Sector has up to 4 boundaries with neighbouring Sectors (fewer when the sector is in a corner or along the edge of the game world). Each boundary can contain Portals which indicate a navigable point from the current Sector to a neighbour. Portals serve a dual purpose, one of which is to provide responsiveness - `FlowFields` may take time to generate so when an actor needs to move a quick A* pathing query can produce an initial path route based on moving from one Portal to another and they can start moving in the general direction to the goal/target/endpoint. Once the `FlowFields` have been built the actor can switch to using them for granular navigation instead.

The following sectors are located away from any edges of the world which means each boundary can have Portals (the purple cells) and these sectors are neighbours:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/portals.png" alt="portals" width="400" height="500"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/portals_adj.png" alt="portals" width="400" height="500"/>

A Portal is generated at the midpoint of a boundary - in situations where the `CostField` contains `255` costs along the edge then multiple Portals may be generated at the midpoint of each valid pathable segment along the boundary. This is propagated to neighbouring Sectors so that every Portal has a neighbour buddy (as evident in the right hand Sector above, `S(1, 1)` portal `(9, 1)` allows movement into `S(2, 1)` portal `(0, 1)`, even though `S(2, 1)` has a whole boundary that appears completely pathable).

On a larger scale (but still small) and for the simplest `CostField` available, a `2x2` Sector grid produces predictable boundary Portals.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/sectors_portals.png" alt="sector_portals" width="400" height="400"/>

Every Portal has the concept of a Window. A Window describes the size of the Portal, i.e how long it is before it hits a wall or world boundary. This is used later in the `IntegrationField` so that flows don't concentrate towards a Portal itself, instead they converge along the Window towards the neighbour Sector.

Example of a Portal `P`, with a Window `w`, covering the boundary segment:
<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/portal_window.png" alt="pw"/>

### Portal Graph

For finding a path from one Sector to another, at a Portal level, all Sector Portals are stored within a graph. The Portals are stored as Nodes and Edges are created between them to represent traversable paths, it gets built in three stages:

1. For all Portals add a graph `node`
2. For each sector create `edges` (pathable routes) to and from each Portal `node` - effectively create internal walkable routes of each sector
3. Create `edges` across the Portal `nodes` on all sector boundaries - effectively create external walkable routes to join sectors together

This allows the graph to be queried with a `source` sector and a `target` sector and a list of Portals are returned which can be pathed. When a `CostField` is changed this triggers the regeneration of the sector Portals for the region that `CostField` resides in (and its neighbours to ensure homogenous boundaries) and the graph is updated with any new Portals `nodes` and the old ones are removed.

</details>

## IntegrationField

<details>
<summary>Click to expand!</summary>

An `IntegrationField` is an array of 32-bit values. It uses the `CostField` to produce a cumulative cost to reach the end goal/target. It's an ephemeral field, as in it gets built for a required sector and then consumed by the `FlowField` calculation. The first 16-bits of each field cell value are used for a cost measurement while the second 16-bits are used as flags to indicate certain properties of a cell. The flags are classified as:

* INT_BITS_LOS - indicates Line Of Sight from the cell to the goal cell
* INT_BITS_GOAL - indicates the cell is the goal
* INT_BITS_CORNER - indicates a point where Line Of Sight may be broken and is used to discover which cells should be marked as `INT_BITS_WAVE_BLOCKED`
* INT_BITS_WAVE_BLOCKED - marks cells to prevent Line Of Sight from being propagated around corners
* INT_BITS_PORTAL - marks cells that are portals between sectors
* INT_BITS_IMPASSABLE - marks a cell that cannot be pathed through

When a new route needs to be processed the first 16-bits of the field values are set to `u16::MAX` and the field cell containing the goal is set to `0`. Any cells which are impassable in the `CostField` are marked in the `IntegrationField` with their second 16-bits as `INT_BITS_IMPASSABLE`.

The `IntegrationField` is built from a number of passes:

### 1. Line Of Sight Pass

In order to reduce needless pathfinding near the goal a Line Of Sight (LOS) pass is performed from the goal Sector. The idea being that if an actor moves into a field cell that has LOS then it no longer needs to follow the FlowFields and can instead directly path to the goal.

The LOS phase begins as a wavefront from the goal that interrogates the adjacent neighbouring field cells. If an adjacent cell is not marked as impassable then it must have LOS to the goal and the value of the cell receives a wavefront cost plus the LOS bit flag. The wavefront then expands (whereby the wavefront cost increments by 1) to interrogate the adjacent cells of the neighbours and repeats until the wavefront cannot propagate any further.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_los_prop0.png" alt="iflp0"/>

As the wavefront expands it may encounter an impassable field cell (a block box in the diagrams). This causes two things to happen:

First, wavefront expansion cannot continue in the direction of the impassable field cell so it is removed from being a candidate in the next round of wavefront propagation.

Second, if there is a vacant field cell next to the impassable field cell then this indicates a Corner. A Corner means that LOS will be blocked in a given direction and the Corner is recorded for stage 2, the integrated cost calculation.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_los_prop1.png" alt="iflp1"/>

By taking a vector from the starting goal to the corner we can then extend this vector to calculate what field cells lie along a line. The field cells on this line are stored as corners and are updated with the flag for WavefrontBlocked. Meaning that as LOS expands and propagates, if a WavefrontBlocked cell is encountered then the cell is removed as a candidate in further LOS propagation. This ensures that LOS cannot flow around impassable areas.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_los_prop2.png" alt="iflp2"/>
<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_los_prop3.png" alt="iflp3"/>

Any available LOS propagation continues until all possible cells are exhausted:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_los_prop4.png" alt="iflp4"/>

Once the wavefront has exhausted expansion from either hitting the sector boundaries or from impassable cells/corners we can then calculate the actual integrated cost of the field.

### 2. Integrated Cost Calculation

From the Corners of an `IntegrationField` recorded previously we start a new series of wavefronts that radiate from the corners considering any adjacent field cells that have not been marked as LOS or impassable.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_cost_prop0.png" alt="ifcp0"/>

To calculate the cost of the cells in the field:

1. The valid cardinal neighbours of the corners are determined (one, none or many of North, East, South, West)
2. For each neighbour field cell lookup their `CostField` value
3. Add their cost to the wavefronts current integration cost to determine each neighbours integrated cost value (example below, at the corner the wavefront cost assigned was `4`, assuming the `CostField` value of the adjacent cell is `1` then the integrated cost becomes `5`)

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_cost_prop1.png" alt="ifcp1"/>

4. Wavefront propagates to the next neighbours, find their integrated costs by repeating steps 1-3, repeat, repeat, until the wavefront can no longer propagate

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_cost_prop2.png" alt="ifcp2"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_cost_prop3.png" alt="ifcp3"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_cost_prop4.png" alt="ifcp4"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_cost_prop5.png" alt="ifcp5"/>

The end result effectively produces a gradient of high numbers to low numbers, a <i>flow</i> of sorts.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_cost_prop6.png" alt="ifcp6"/>

For Sectors other than the goal the process is effectively the same where boundary portals are treated as corners and wave propagation expanded.

NB: the following diagrams use smaller sector sizes and exclude LOS but demonstrate how integrated cost is accumulated and creates a gradient from portal to portal

From the graph of `Portals` we can get a path of `Portals` to guide the actor over several sectors to the desired sector, the `IntegrationField` of the goal sector has been calculated so next we "hop" through the boundary `Portals` working backwards from the goal sector to the actor sector (Portals are denoted as a purple shade) to produce a series of `IntegrationFields` for the chaining Sectors describing the flow movement.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_sector_to_sector_0.png" alt="ifsts0" width="260" height="310"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_sector_to_sector_1.png" alt="ifsts1" width="260" height="310"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_sector_to_sector_2.png" alt="ifsts2" width="260" height="310"/>

In terms of pathfinding the actor will favour flowing "downhill". From the position of the actor and looking at its field cell neighbours a smaller value in that sectors `IntegrationField` means a more favourable point for reaching the end goal, going from smaller to smaller values, basically a gradient flowing downhill to the destination.

This informs the basis of a `FlowField`.

As an example for a `30x30` world (manually calculated and predates the work on the Window concept of portals), goal at `0` with an actor at `A`, an `IntegrationField` set interrogating all sector `Portals` may produce a set of fields looking similar to:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_prop_big_example.png" alt="ifpbe" width="75%"/>

Notice the cool waves that propagate out from the goal!

Generating the fields for this path programmatically leads to (we don't bother generating fields for sectors the actor isn't pathing through):

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/generated_int_fields.png" alt="gif" width="75%"/>

From the `IntegrationFields` we can now build the final set of fields - `FlowFields`

</details>

## FlowField

<details>
<summary>Click to expand!</summary>

A `FlowField` is an array of 8-bit values built from a Sectors `IntegrationField`. The first 4 bits of the value correspond to one of eight compass movement directions an actor can take (plus a zero vector when impassable) and the second 4 bits correspond to flags which should be used by a character controller/steering pipeline to follow a path.

The directional bits are defined as:

* `0b0000_0001` - North
* `0b0000_0010` - East
* `0b0000_0100` - South
* `0b0000_1000` - West
* `0b0000_0011` - North-East
* `0b0000_0110` - South-East
* `0b0000_1100` - South-West
* `0b0000_1001` - North-West
* `0b0000_0000` - default on `FlowField` initialisation, is always replaced by other values

The assistant flags are defined as:

* `0b0001_0000` - pathable
* `0b0010_0000` - has line-of-sight to goal, an actor no longer needs to follow the field, it can move in a straight line to the goal. This avoids calculating field values that aren't actually needed and once an actor enters a cell with this flag then they no longer need to spend time looking up a `FlowField`
* `0b0100_0000` - indicates the goal
* `0b1000_0000` - indicates a portal goal leading to the next sector
* `0b1110_0000` - marks impassable

So a field cell in the `FlowField` with a value of `0b0001_0110` means the actor should flow in the South-East direction. In terms of use don't worry about understanding these bit values too much, the [Usage](#usage) section shows the helpers for interpreting the values of the `FlowField` to steer an actor.

Using the `IntegrationFields` generated before, with an actor in the top right trying to reach the bottom left, we now generate the `FlowFields`:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/generated_flow_fields.png" alt="gff"/>

The thinner portion of each cell icon indicates the flow direction. The actor runs along the flow lines leading to the goal. This means for a group of actors they will flow towards the goal with a formation-like behaviour along the flow lines.

</details>

## FlowField Cache

<details>
<summary>Click to expand!</summary>

To enable actors to reuse `FlowFields` (thus avoiding repeated calculations) they are stored in a cache.

`FlowFieldCache` - for every sector-to-portal part of a route a `FlowField` is built and stored in the cache. Actors can poll this cache to get the true flow direction to their goal. A Character Controller/Steering Pipeline is responsible for interpreting the values of the `FlowField` to produce movement - while this plugin includes a basic Steering Pipeline in the examples the reality is that every game has it's own quirks and desires for movement so you will most likely want to build your own Pipeline. The real point of this plugin is to encapsulate the data structures and logic to make a `FlowField` which an Actor can then read through it's own implementation.

</details>

## Actor Sizes

<details>
<summary>Click to expand!</summary>

In a simulation you may have actors of different sizes and a gap between impassable walls, consider these purple actors:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/actor_size_pre.png" alt="asp" width="300"/>

The smaller actor on the left can evidently pass through the gap between the impassable terrain. On the right however the actor is much larger and as such when processing a path request only routes with suitable clearance should be considered (otherwise with a collision system in place it'd just bump into the walls to the side and never make it through).

To handle this the overall `Dimensions` of the world makes use of an `actor_radius` parameter. This radius is used to scale the impassable/wall cells of `CostFields` to close any gaps that are smaller than the actor.

In terms of what an actor 'sees' after requesting a route, the smaller actor on the left can path through the gap whereas the larger actor on the right would search for an alternate route:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/actor_size_post.png" alt="aspo" width="300"/>

In a game with actors of multiple sizes you will want to create distinct entities from `FlowFieldTiles` where each is configured to handle a certain size of actor.

Example, mark actors with a size and have them request paths from a `FlowFieldTiles` that corresponds to their size:
```rust
#[derive(Component)]
struct ActorSmall
#[derive(Component)]
struct ActorLarge

fn setup () {
    cmds.spawn(FlowFieldTiles::new(
    	/* stuff */
    )).insert(ActorSmall);

    let actor_size_large = 78.0;
    cmds.spawn(FlowFieldTiles::new(
    	/* stuff */
    )).insert(ActorLarge);
}

fn system_navigation_small_actors(
    actor_q: Query<&Actor, With<ActorSmall>>,
    field_q: Query<&FlowFieldTiles, With<ActorSmall>>
) {/* handling movement etc */}

fn system_navigation_large_actors(
    actor_q: Query<&Actor, With<ActorLarge>>,
    field_q: Query<&FlowFieldTiles, With<ActorLarge>>
) {/* handling movement etc */}
```

</details>
</br>

# Usage

Update your `Cargo.toml` and add any features you require, to actually interface with calculated fields you should enable either `2d` or `3d` depending on the coordinate system of your world:

```toml
[dependencies]
bevy_flowfield_tiles_plugin = { version = "0.x", features = ["2d"] }
```

## Default

Add the plugin to your app:

```rust
use bevy_flowfield_tiles_plugin::prelude::*;

fn main() {
    App::new()
        // ... snip
        .add_plugins(FlowFieldTilesPlugin)
        // ... snip
}
```

NB: the internal systems of the plugin run inside the `PostUpdate` schedule. Any actor or your own system that needs to access the `FlowFieldTiles` component should NOT attempt access within `PostUpdate`.


## Initialising Data

To create a `FlowFieldTiles` you need to know up front:
- Origin: the point in space calculations are centred on
- World size: the dimensions of your world. In `3d` this is the `x-z` length of gameplay space, in `2d` this is the `x-y` length
- World Unit Size: the dimension of a unit of space. For example in `3d` this might be `1` for a metre, for `2d` this could be `64` pixels
- Actor Radius: the size of the actors. Should not exceed `world_unit_size * 10`

Spawning an entity with the component:

```rust
fn my_system(mut cmds: Commands) {
		let origin = (0.0, 0.0);
		let size = (1920.0, 1920.0);
		let world_unit_size = 64.0;
		let actor_radius = 24.0;
    cmds.spawn(FlowFieldTiles::new(origin, size, world_unit_size, actor_radius));
}
```

Note that this will initialise all the `CostFields` representing the world with cell values of `1`. Meaning everywhere is pathable, in all likelihood you'll then need to seed the fields with true values.

To modify a `CostField` the methods `FlowFieldTiles::add_costfield_update_2d()` and `FlowFieldTiles::add_costfield_update_3d()` can be used to set a cost for a cell in space. For instance you may create a default `FlowFieldTiles` and as part of loading a player into a game world you then perform a series of updates based on world geometry.

Alternatively the features `ron` and `heightmap` may be of used to create `FlowFieldTiles` from some data.

## Path Request

NB: this touches on a Steering Pipeline and is purely a basic example. This crate focuses on the FlowFields algorithm and an agnostic Steering Pipeline is out of scope.

When interacting with the algorithm an actor needs to store certain data:

```rust
#[derive(Default, Component)]
pub struct Pathing {
	/// The place an actor wants to go
	target: Option<Vec2>,
	/// A task received from FlowFieldTiles that can be polled to see if a route is available
	pollable_route: Option<Task<Option<Vec<RouteStep>>>>,
	/// The generated route that can be used to retrieve the FlowFields
	route: Option<Vec<RouteStep>>,
	/// If CostFields have changed since an actor got their RouteSteps, then it is possible
	/// that a RouteStep could be out of date. I.e there's no longer a FlowField for it.
	/// If so an actor, failing to get a FlowField, can count ticks and if X number pass they
	/// should begin re-requesting a route
	request_ticks: u32,
}
```

An actor should set a `target` (this could be from a mouse click or from some system producing actor behaviour):

```rust
fn click_set_target(
	mouse_button_input: Res<ButtonInput<MouseButton>>,
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	mut actor_q: Query<&mut Pathing, With<Actor>>,
) {
	if mouse_button_input.just_released(MouseButton::Right) {
		// get 2d world position of cursor
		let (camera, camera_transform) = camera_q.single().unwrap();
		let window = windows.single().unwrap();
		let Some(cursor_position) = window.cursor_position() else {
			return;
		};
		let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position)
		else {
			return;
		};
		// set the actor target and abandon any existing pollable task
		let mut actor_pathing = actor_q.single_mut().unwrap();
		let existing_route = &mut actor_pathing.pollable_route;
		*existing_route = None;
		actor_pathing.target = Some(world_position);
		actor_pathing.route = None;
	}
}
```

An actor that has a `target` should query `FlowFieldTiles` for a task they can poll:

```rust
fn actor_request_route(
	mut actor_q: Query<(&Transform, &mut Pathing), With<Actor>>,
	flow_q: Query<&FlowFieldTiles>,
) {
	// get the actor position
	for (actor_tform, mut actor_pathing) in &mut actor_q {
		// only proceed if they have a target and don't already have a route/task
		if let Some(target) = actor_pathing.target
			&& actor_pathing.route.is_none()
			&& actor_pathing.pollable_route.is_none()
		{
			// ask for a route
			for flowfield_tiles in &flow_q {
				let task = flowfield_tiles.get_route_2d(actor_tform.translation.truncate(), target);
				// store the task so the actor can poll it for a finished route.
				// for performance routes are generated in an AsyncTaskPool,
				// so we need to poll the task later to see that it is finished
				if let Some(t) = task {
					actor_pathing.pollable_route = Some(t);
					actor_pathing.route = None;
				}
			}
		}
	}
}
```

An actor with a task can then poll it to get the sector-sector route once it has finished computing:

```rust
fn actor_update_route(mut actor_q: Query<&mut Pathing, With<Actor>>) {
	for mut pathing in &mut actor_q {
		if let Some(mut poll) = pathing.pollable_route.as_mut()
			&& let Some(route) = check_ready(&mut poll)
		{
			// task finished
			pathing.pollable_route = None;
			pathing.route = route;
		}
	}
}
```

An actor now has a route, they can query `FlowFieldTiles` to get the `FlowField` for their current position and use it for steering:

```rust
fn actor_steering(
	mut actor_q: Query<(&mut LinearVelocity, &mut Transform, &mut Pathing), With<Actor>>,
	flow_q: Query<&FlowFieldTiles>,
	time_step: Res<Time>,
) {
	let flowfield_tiles = flow_q.single().unwrap();
	for (mut velocity, tform, mut pathing) in actor_q.iter_mut() {
		// only proceed for actors with a route
		if let Some(steps) = &mut pathing.route {
			if let Some(step) = steps.first() {
				// get actor position in terms of sector and cell
				let actor_pos = tform.translation.truncate();
				let Some((sector, cell)) = flowfield_tiles
					.get_dimensions()
					.get_sector_and_field_cell_from_xy(actor_pos)
				else {
					// actor is outside game world, do something about it...
					continue;
				};
				// is actor in the sector of the current route step
				if *step.get_sector() == sector {
					// attempt to get the FlowField, the field is built inside of
					// an AsyncTaskPool so it may take a moment for it to become
					// available
					if let Some(field) = flowfield_tiles.read_flowfield(step) {
						if field.has_los(&cell) {
							// has LOS can move straight to goal
							let dir = (pathing.target.unwrap() - actor_pos).normalize();
							velocity.0 = dir * SPEED * time_step.delta_secs();
						} else {
							// read cell bits to get dir to move
							if let Some(dir) = field.get_2d_dir(&cell) {
								// move along the flow
								velocity.0 = dir * SPEED * time_step.delta_secs();
							}
						}
					} else {
						// if a costfield has been changed then the RouteStep may no longer
						// be valid, meaning no FlowField will be generated for it.
						// count ticks and if too many remove route so a new request
						// will be sent
						pathing.request_ticks += 1;
						if pathing.request_ticks > 300 {
							pathing.request_ticks = 0;
							pathing.pollable_route = None;
							pathing.route = None;
						}
					}
				} else {
					// actor is not in the sector denoted by the RouteStep,
					// schedule first step removal as they may of moved over the 
					// sector boundary so they should now read the next RouteStep
					steps.remove(0);
				}
			} else {
				// steps is empty meaning it is exhausted. This might happen if an actor
				// has a collision and is knocked into a sector not along their path.
				// The actor should prepare to request a new route
				// Setting route to None while pathing.target is still set will cause
				// a new request for a route to be sent in a different system
				pathing.route = None;
			}
		}
	}
}
```

NB: when a CostField is modified Portals and the graph are updated and any RouteStep or FlowField involving the modified Sector CostField are removed - actors need to request a new route if they repeatedly fail to retrieve a FlowField as their tracking RouteStep could be out of date if CostFields have changed.

### Things that may throw the pathing off

If you're combining this with a Physics simulation you'll need to ensure that your CharacterController is very robust, consider some scenarios that may happen:

* A moving actor collides with something that bounces it into a sector which is not part of its route. How can the actor be made aware that this has happened and request a new route?
* An actor has escaped/tunnelled outside of your world (its translation exceeds the bounds of Dimensions), should it be despawned or relocated to be within the bounds?

# Features

* `serde` - enables serialization on some data types
* `ron` - enables reading `CostField` from files. NB: fixed-size arrays in `.ron` are written as tuples
* `2d` - enables interface methods when working with Flowfields in a 2d world, additionally allows using a list of Bevy 2d meshes to initialise the Flowfields
* `3d` - enables interface methods when working with FlowFields in a 3d world
* `heightmap` - allows initialising the `CostField`s from a greyscale png/jpeg where each pixel of the image represents a `FieldCell`. Alpha channel is optional (it'll just be ignored if included in the image). A pixel with colour channels `(0, 0, 0, 255)` (black) represents an impassable `255` cost whereas `(255, 255, 255, 255)` (white) is translated as a cost of `1`, channel values in between will be more expensive costs

# Performance

Benchmarks are split into two categories:

* Data initialisation
  * [init_costfields](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/init_costfields.rs) - measures the time it takes to initialise 100x100 sector `CostFields`
  * [init_portals](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/init_portals.rs) - measures the time it takes to build `Portals` across 100x100 sectors
  * [init_bundle](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/init_bundle.rs) - measure the total time it takes to have the `FlowFieldTiles` ready

* Algorithm use - measures generating a set of FlowFields
  * [calc_route](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/calc_route.rs) - measures how long it takes to generate a route from one corner of a 100x100 sector layout to the opposite corner
  * [calc_flow_open](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/calc_flow_open.rs) - measure how long it takes to create a full set of `FlowFields` describing movement across uniform `CostFields` (cost = 1) from one corner to another
  * [calc_flow_sparse](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/calc_flow_sparse.rs) - measure how long it takes to create a full set of `FlowFields` describing movement across a variety of sectors containing clumps of impassable tiles
  * [calc_flow_maze](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/calc_flow_maze.rs) - measures how long it takes to create a full set of `FlowFields` describing movement from one corner to another in a 100x100 sector world. The world is composed of vertical corridors meaning that the actor has to path up and down to eventually snake it's way to the goal


On a 12th Gen Intel Core i7-12700H, 2700 MHz; I get (your mileage may vary):
- init_costfields: 92ms
- init_portals: 536ms
- init_bundle: 776ms
- calc_route: 84ms
- calc_flow_open: 3ms
- calc_flow_sparse: 2.9ms
- calc_flow_maze: 283ms, expected as all 100x100 sectors are crossed multiple times so there's 10,000s of fields to generate

# LICENSE

Dual license of MIT and Apache.
