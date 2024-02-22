[![crates.io](https://img.shields.io/crates/v/bevy_flowfield_tiles_plugin)](https://crates.io/crates/bevy_flowfield_tiles_plugin)
[![docs.rs](https://docs.rs/bevy_flowfield_tiles_plugin/badge.svg)](https://docs.rs/bevy_flowfield_tiles_plugin)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/blondeburrito/bevy_flowfield_tiles_plugin#license)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/blondeburrito/bevy_flowfield_tiles_plugin/ci.yml)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/blondeburrito/bevy_flowfield_tiles_plugin/code-cov.yml?label=CodeCov>85%)

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/emblem.png" alt="e" width="300"/>

# bevy_flowfield_tiles_plugin

Inspired by the work of [Elijah Emerson](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter23_Crowd_Pathfinding_and_Steering_Using_Flow_Field_Tiles.pdf) and with inspiration from [leifnode](https://leifnode.com/2013/12/flow-field-pathfinding/) and [jdxdev](https://www.jdxdev.com/blog/2020/05/03/flowfields/) this is an attempt to implement the data structures and logic required to generate a Flowfield representation of a world which can be used to pathfind movable actors.

| bevy | bevy_flowfield_tiles_plugin |
|------|-----------------------------|
| 0.13 |  0.7  |
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

For larger and larger environemnts with an increasing number of pathing actors it may be beneficial to adopt a FlowField based approach due to the data sharing and formation/group like movement it promotes. FlowField Tiles are complex, it's effectively akin to fluid mechanics, so this is an attempt to bring an agnostic implementation to the [Bevy](https://github.com/bevyengine/bevy/tree/main) game engine. My motivation for this is that I recently implemented a Way-point Graph for a prototype. In order to provide 'ok' actor movement it had to be made from 16 million data points. To prevent an actor from occasionally zig-zagging across the game world the granularity had to be boosted to 80 million data points to create a 'lifelike' impression of movement. That was just silly so I began looking into the history of pathfinding whereupon I stumbled across FlowField Tiles and decided to try and implement it with my favourite langauge and engine.

## Useful Definitions

* Sector - a slice of a game world composed of three 2D arrays called fields (`CostField`, `IntegrationField` and `FlowField`). A game world is effectively represented by a number of Sectors
* CostField - a 2D array describing how difficult it is to path through each cell of the array. It is always present in system memory
* Cost - how difficult/expensive it is to path somewhere, you could also call it <i>weight</i>, each cell of `CostField` has one of these
* Portal - a navigatable point which links one Sector to another to enable movement from one side of the world to another
* IntegrationField - a 2D array which uses the CostField to determine a cumulative cost of reaching the goal/endpoint (where you want to path to). This is an ephemeral field - it exists when required to calculate a `FlowField`
* FlowField - a 2D array built from the `IntegrationField` which decribes how an actor should move (flow) across the world
* FlowField Cache - a means of storing `FlowFields` allowing multiple actors to use and reuse them
* Ordinal - a direction based on traditional compass ordinals: N, NE, E, SE, S, SW, W, NW. Used for discovery of Sectors/field cells at various points within the algorithm
* Field cell - an element of a 2D array
* Goal - the target field cell an actor needs to path to
* Portal goal - a target point within a sector that allows an actor to transition to another sector, thus bringing it closer towards/to the goal

# Design/Process

<details>
<summary>Click to expand!</summary>

To generate a set of navigation `FlowFields` the game world is divided into Sectors indexed by `(column, row)` and each Sector has 3 layers of data: `[CostField, IntegrationField, Flowfield]`. Each layer aids the next in building out a path. A concept of `Portals` is used to connect Sectors together.

## Sector

<details>
<summary>Click to expand!</summary>

For a 3-dimensional world the `x-z` (`x-y` in 2d) plane defines the number of Sectors used to represent it with a scale factor called `sector_resolution`. This means that a for a `(30, 30)` world with a resolution of `10` there would be `3x3` Sectors representing it - this implies that a single sector has relative dimensions of `(10, 10)` and a single field cell within a sector represents a `1x1` unit area. Each Sector has an associated unqiue ID taken as its position: `(column, row)`.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/sectors.png" alt="sectors" width="250"/>

Likewise for a `(300, 550)` resolution `10` world you'll be looking at `30` columns and `55` rows. The advantage of dividing a world into Sectors (as opposed to treating the whole world as a giant `Flowfield`) is that the work in generating a path can be split into multiple operations and only touch certain sectors. Say for the `(300, 550)` world you do treat it as a single set of fields - when calculating a path you could potentially have to calculate the Flowfield values for `165,000` field cells. Splitting it into sectors may mean that your path only takes you through 20 sectors, thereby only requiring `2,000` `Flowfield` field cells to be calculated.

</details>

## CostField

<details>
<summary>Click to expand!</summary>

A `CostField` is an `MxN` 2D array of 8-bit values, by default this is always a `10x10` array. The values indicate the `cost` of navigating through that cell of the field. A value of `1` is the default and indicates the easiest `cost`, and a value of `255` is a special value used to indicate that the field cell is impassable - this could be used to indicate a wall or obstacle. All other values from `2-254` represent increasing cost, for instance a slope or difficult terrain such as a marsh. The idea is that the pathfinding calculations will favour cells with a smaller value before any others.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/cost_field.png" alt="cf" width="370"/>

At runtime the `CostField` is generated for each Sector with the default value - although with the feature `ron` it is possible to load the fields from disk, or with the feature `heightmap` a greyscale png/jpeg can be used to seed the fields. See the [Usage](#usage) section below for details on updating the `CostFields` during an inital pass (i.e when loading a level) and tweaking it during gameplay for a world which dynamically evolves with obstacles (flipping a cell to to a higher cost or an impassable `255` when something like a wall is placed or the ground splits into a fissure).

This array is used to generate the `IntegrationField` when requesting a navigatable path.

</details>

## Portals

<details>
<summary>Click to expand!</summary>

Each Sector has up to 4 boundaries with neighbouring Sectors (fewer when the sector is in a corner or along the edge of the game world). Each boundary can contain Portals which indicate a navigatable point from the current Sector to a neighbour. Portals serve a dual purpose, one of which is to provide responsiveness - `FlowFields` may take time to generate so when an actor needs to move a quick A* pathing query can produce an inital path route based on moving from one Portal to another and they can start moving in the general direction to the goal/target/endpoint. Once the `FlowFields` have been built the actor can switch to using them for granular navigation instead.

The following sectors are located away from any edges of the world which means each boundary can have Portals (the purple cells):

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/portals.png" alt="portals" width="400" height="500"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/portals_adj.png" alt="portals" width="400" height="500"/>

A Portal is generated at the midpoint of a boundary - in situations where the `CostField` contains `255` costs along the edge then multiple Portals may be generated at the midpoint of each valid pathable segment along the boundary and this is propagated to neighbouring Sectors so that every Portal has a neighbour buddy (as evident in the right hand Sector above, `S(1, 1)` portal `(9, 1)` allows movement into `S(2, 1)` portal `(0, 1)`, even though `S(2, 1)` has a whole boundary that appears completely pathable).

On a larger scale (but still small) and for the simplist `CostField` available, a `2x2` Sector grid produces predictable boundary Portals.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/sectors_portals.png" alt="sector_portals" width="400" height="400"/>

### Portal Graph

For finding a path from one Sector to another at a Portal level all Sector Portals are recorded within a data strucutre known as `PortalGraph`. The Portals are stored as Nodes and Edges are created between them to represent traversable paths, it gets built in three stages:

1. For all Portals add a graph `node`
2. For each sector create `edges` (pathable routes) to and from each Portal `node` - effectively create internal walkable routes of each sector
3. Create `edges` across the Portal `node` on all sector boundaries (walkable route from one sector to another)

This allows the graph to be queried with a `source` sector and a `target` sector and a list of Portals are returned which can be pathed. When a `CostField` is changed this triggers the regeneration of the sector Portals for the region that `CostField` resides in (and its neighbours to ensure homogenous boundaries) and the graph is updated with any new Portals `nodes` and the old ones are removed.

</details>

## IntegrationField

<details>
<summary>Click to expand!</summary>

An `IntegrationField` is an `MxN` 2D array of 16-bit values. It uses the `CostField` to produce a cumulative cost to reach the end goal/target. It's an ephemeral field, as in it gets built for a required sector and then consumed by the `FlowField` calculation.

When a new route needs to be processed the field values are set to `u16::MAX` and the field cell containing the goal is set to `0`.

A series of passes are performed from the goal as an expanding wavefront calculating the field values:

1. The valid ordinal neighbours of the goal are determined (North, East, South, West - when not against a sector/world boundary)
2. For each ordinal field cell lookup their `CostField` value
3. Add the `CostField` cost to the `IntegrationFields` cost of the current cell (at the beginning this is the goal int cost `0`)
4. Propagate to the next neighbours, find their ordinals and repeat adding their cost value to to the current cells integration cost to produce their cumulative integration cost, and repeat until the entire field is done

This produces a nice diamond-like pattern as the wave expands (the underlying `CostField` is set to `1` here):

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_prop0.png" alt="ifp0" width="300" height="310"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_prop1.png" alt="ifp1" width="300" height="310"/>
<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_prop2.png" alt="ifp2" width="300" height="310"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_prop3.png" alt="ifp3" width="300" height="310"/>

Now a dimaond-like wave isn't exactly realistic in a world of dynamic movement so at some point it should be replaced, based on various articles out there it seems people adopt the [Eikonal equation](https://en.wikipedia.org/wiki/Eikonal_equation) to create a more spherical wave expanding over the field space.

When it comes to `CostField` containing impassable markers, `255` as black boxes, they are ignored so the wave flows around those areas:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_prop_impassable.png" alt="ifpi" width="300" height="310"/>

And when your `CostField` is using a range of values to indicate different areas to traverse, such as a steep hill:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/cost_field_hill.png" alt="cfh" width="300" height="310"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_prop_hill.png" alt="ifph" width="300" height="310"/>

So this encourages the pathing algorithm around obstacles and expensive areas in your world!

This covers calculating the `IntegrationField` for a single sector containing the goal but of course the actor could be in a sector far away, this is where `Portals` come back into play.

From the `PortalGraph` we can get a path of `Portals` to guide the actor over several sectors to the desired sector, extending the above the `IntegrationField` of the goal sector has been calculated so next we "hop" through the boundary `Portals` working backwards from the goal sector to the actor sector (Portals are denoted as a purple shade) to produce a series of `IntegrationFields` for the chaining Sectors describing the flow movement.

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_sector_to_sector_0.png" alt="ifsts0" width="260" height="310"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_sector_to_sector_1.png" alt="ifsts1" width="260" height="310"/><img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_sector_to_sector_2.png" alt="ifsts2" width="260" height="310"/>

In terms of pathfinding the actor will favour flowing "downhill". From the position of the actor and looking at its field cell neighbours a smalller value in that sectors `IntegrationField` means a more favourable point for reaching the end goal, going from smaller to smaller values, basically a gradient flowing downhill to the destination.

This informs the basis of a `FlowField`.

As an example for a `30x30` world, goal at `0` with an actor at `A`, an `IntegrationField` set interrogating all sector `Portals` may produce a set of fields looking similar to:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/int_field_prop_big_example.png" alt="ifpbe" width="75%"/>

Notice the cool waves that propagate out from the goal!

Generating the fields for this path programmatically leads to:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/generated_int_fields.png" alt="gif" width="75%"/>

Notice that we don't bother generating the fields for sectors the actor doesn't need to path through. Also a Portal represents the midpoint of a traversable sector boundary, when generating the field we expand the portal to cover its entire segment - this increases efficiency so that an actor can more directly approach its goal rather than zig-zagging to portal boundary points.

From the `IntegrationFields` we can now build the final set of fields - `FlowFields`

</details>

## FlowField

<details>
<summary>Click to expand!</summary>

A `FlowField` is an `MxN` 2D array of 8-bit values built from a Sectors `IntegrationField`. The first 4 bits of the value correspond to one of eight ordinal movement directions an actor can take (plus a zero vector when impassable) and the second 4 bits correspond to flags which should be used by a character controller/steering pipeline to follow a path.

The directional bits are defined as:

* `0b0000_0001` - North
* `0b0000_0010` - East
* `0b0000_0100` - South
* `0b0000_1000` - West
* `0b0000_0011` - North-East
* `0b0000_0110` - South-East
* `0b0000_1100` - South-West
* `0b0000_1001` - North-West
* `0b0000_0000` - zero vector, represents impassable cells
* `0b0000_1111` - default on `FlowField` initialisation, is always replaced by other values

The assistant flags are defined as:

* `0b0001_0000` - pathable
* `0b0010_0000` - has line-of-sight to goal, an actor no longer needs to follow the field, it can move in a straight line to the goal. This avoids calculating field values that aren't actually needed and once an actor enters a cell with this flag then they no longer need to spend time looking up a `FlowField``
* `0b0100_0000` - indicates the goal
* `0b1000_0000` - indicates a portal goal leading to the next sector

So a field cell in the `FlowField` with a value of `0b0001_0110` means the actor should flow in the South-East direction. In terms of use don't worry about understanding these bit values too much, the [Usage](#usage) section shows the helpers for interpreting the values of the `FlowField` to steer an actor.

Using the `IntegrationFields` generated before, with an actor in the top right trying to reach the bottom left, we now generate the `FlowFields`:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/generated_flow_fields.png" alt="gff"/>

The thinner porition of each cell icon indicates the flow direction. The actor runs along the flow lines leading to the goal. This means for a group of actors they will flow towards the goal with a formation-like behaviour along the flow lines.

</details>

## Route & FlowField Cache

<details>
<summary>Click to expand!</summary>

To enable actors to reuse `FlowFields` (thus avoiding repeated calculations) a pair of caches are used to store pathing data:

1. Route Cache - when an actor requests to go somewhere a high-level route is generated from describing the overall series of sector-portals to traverse (`PortalGraph` A*). If a `FlowField` hasn't yet been calculated then an actor can use the `route_cache` as a fallback to gain a generalist direction they should start moving in. Once the `FlowFields` have been built they can swap over to using those more granular paths. TODO: ~~Additionally changes to `CostFields` can change portal positions and the real best path, so `FlowFields` are regenerated for the relevant sectors that `CostFields` have modified and during the regeneration steps an actor can once again use the high-level route as the fallback~~

1. Field Cache - for every sector-to-portal part of a route a `FlowField` is built and stored in the cache. Actors can poll this cache to get the true flow direction to their goal. A Character Controller/Steering Pipeline is responsible for interpreting the values of the `FlowField` to produce movement - while this plugin includes a Steering Pipeline the reality is that every game has it's own quirks and desires for movement so you will most likely want to build your own Pipeline. The real point of this plugin is to encapulsate the data structures and logic to make a `FlowField` which an Actor can then read through it's own implementation.

Note that the data stored in the caches is timestamped - if a record lives longer than 15 minutes then it is purged to reduce size and improve lookup efficiency. When implemnting a steering pipeline/character controller to interpret the `FlowFields` you may need to account for these old routes/paths expiring.

</details>

## Actor Sizes

<details>
<summary>Click to expand!</summary>

In a simulation you may have actors of different sizes and a gap between impassable walls, consider these purple actors:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/actor_size_pre.png" alt="asp" width="300"/>

The smaller actor on the left can evidently pass through the gap between the impassable terrain. On the right however the actor is much larger and as such when processing a `PathRequest` only routes with suitable clearance should be considered (otherwise with a collision system in place it'd just bump into the walls to the side and never make it through).

To handle this the overall `MapDimenions` component which defines the sizing of the various fields contains an `actor_scale` parameter. This scaling is determined by the actor size and unit-size of a cell within a field. For instance a Sector with pixel dimensions of `640x640` means that each cell in the `(m, n) -> (10, 10)` fields represents a pixel area of `64x64`, if an actor is larger than `64` pixels in width then a ratio between actor size and cell size is applied to 'grow' impassable cells to close off gaps that would be too small for the actor to path through.

In terms of what an actor 'sees' after requesting a route, the smaller actor on the left can path through the gap whereas the larger actor on the right would search for an alternate route:

<img src="https://raw.githubusercontent.com/BlondeBurrito/bevy_flowfield_tiles_plugin/main/docs/png/actor_size_post.png" alt="aspo" width="300"/>

In a game with actors of multiple sizes you will want to create distinct entities from `FlowFieldTilesBundle` where each is configured to handle a certain size of actor.

```rust
#[derive(Component)]
struct ActorSmall
#[derive(Component)]
struct ActorLarge

fn setup () {
    let map_length = 1920;
    let map_depth = 1920;
    let sector_resolution = 640;

    let actor_size_small = 16.0;
    cmds.spawn(FlowFieldTilesBundle::new(
        map_length,
        map_depth,
        sector_resolution,
        actor_size_small
    )).insert(ActorSmall);

    let actor_size_large = 78.0;
    cmds.spawn(FlowFieldTilesBundle::new(
        map_length,
        map_depth,
        sector_resolution,
        actor_size_large
    )).insert(ActorLarge);
}

fn system_navigation_small_actors(
    actor_q: Query<&Actor, With<ActorSmall>>,
    field_q: Query<&FlowCache, With<ActorSmall>>
) {/* handling movement etc */}

fn system_navigation_large_actors(
    actor_q: Query<&Actor, With<ActorLarge>>,
    field_q: Query<&FlowCache, With<ActorLarge>>
) {/* handling movement etc */}
```

</details>

</details>
</br>

# Usage

Update your `Cargo.toml` and add any features you require, to actually interface with calculated fields you should enable either `2d` or `3d` depending on the coordinate system of your world:

```toml
[dependencies]
bevy_flowfield_tiles_plugin = { version = "0.x", features = ["3d"] }
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

## Custom System Setup and Constraints

In your own simulation you may well be using custom schedules or stages to control logic execution, the plugin as is sets all the logic to run as part of the `PreUpdate` phase of the main Bevy schedule. To implement the logic into your own scheduling disect the contents of [`plugin/mod.rs`](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/src/plugin/mod.rs) - note that certain systems have been `chained` together and they <b><i>must</i></b> remain chained for accurate paths to be computed.

## Initialising Data

Next it's time to spawn the bundle entity configured to your world size (looking through the examples will give some pointers on this too).

The size and resolution of the world need to be known at initialisation and three values are required:

* `map_length` - in 2d this refers to the pixel `x` size of the world. In 3d this is simply the `x` size
* `map_depth` - in 2d this refers to the pixel `y` size of the world. In 3d this is the `z` size
* `sector_resolution` - determines the numder of sectors by taking each size and dividing them by this value. In 2d this is basically the pixel length of each sector side and likewise for 3d it's the length of each sector side using whatever unit of measurement you've defined (for ease of use I go with a unit of `x` is 1 meter and a unit of `z` is one meter)
  * 2d: a world of pixel size `(1920, 1080)` with a resolution of `40` will produce 48x27 sectors. Another way of looking at this could be based on the idea of having a world made of sprites where each sprite corresponds to where a `FieldCell` would be. If these regular sized sprites have a pixel length and height of `64` and your world is made from a `20x20` grid of these sprites then you can calcualte what the size is. `map_length` would be your sprite length multiplied by the number sprites along the `x` axis of the world, i.e `64 * 20 = 1280`. `map_depth` follows a likewise calculation `64 * 20 = 1280`. As for resolution it will depend on how granular you want, in this example case a `10x10` `CostField` is supposed to overlay an exact number of sprites so we use the sprite size to find the resolution `64 * 10 = 640`.
  * 3d: a world of size `(780x440)` with resolution `10` produces `78x44` sectors. Given that fields are `10x10` arrays this translates to a single `FieldCell` representing a `1x1` unit area

Within a system somewhere you can spawn the Bundle:

```rust
fn my_system(mut cmds: Commands) {
    let map_length = 1920;
    let map_depth = 1920;
    let sector_resolution = 640;
    let actor_size = 16.0;
    cmds.spawn(FlowfieldTilesBundle::new(map_length, map_depth, sector_resolution, actor_size));
}
```

Note that this will initialise all the `CostFields` representing the world with cell values of `1`. Meaning everywhere is pathable, in all likihood you'll then need to seed the fields with true values.

In 3d you could consider making a raycast to the centre of where each FieldCell would be and use something like the `y` position of the ray hit to determine if something is passable or not and then flip the value of that particular `FieldCell` (`EventUpdateCostfieldsCell` can be used to queue a cost change).

Most likely for 2d or more complex 3d scenarios you'll probably want to enable either the `ron`, `csv` or `heightmap` feature which allows for creating a `FlowfieldTilesBundle` with inital `CostFields` from a `.ron` file, a collection of `.csv` or a greyscale png/jpeg where pixel colour channels are translated into costs, the examples showcase this in more detail.

## Path Request

When it comes to interacting with the algorithm this is based on an event to be emitted when a movable actor needs a path:

```rust
struct EventPathRequest {
    /// The starting sector of the request
    source_sector: SectorID,
    /// The starting field cell of the starting sector
    source_field_cell: FieldCell,
    /// The sector to try and find a path to
    target_sector: SectorID,
    /// The field cell in the target sector to find a path to
    target_goal: FieldCell,
}
```

Each parameter can be determined by querying the `MapDimension` component of the Bundle with the starting and end `Transform::translation` of actor position and target position.

Using some example components to track and label an Actor:

```rust
/// Enables easy querying of Actor entities
#[derive(Component)]
struct Actor;
/// Consumed by an Actor steering pipeline to produce movement
#[derive(Default, Component)]
struct Pathing {
    source_sector: Option<SectorID>,
    source_field_cell: Option<FieldCell>,
    target_sector: Option<SectorID>,
    target_goal: Option<FieldCell>,
    portal_route: Option<Vec<(SectorID, FieldCell)>>,
}
```

We can then do something like process mouse clicks to fire off PathRequest events (in 3d use the methods ending in xyz instead):

```rust
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
            // from 2d mouse position get the sector and field cell it is in
            // (if not outside the world)
            let map_dimensions = dimensions_q.get_single().unwrap();
            if let Some((target_sector_id, goal_id)) =
                map_dimensions.get_sector_and_field_cell_from_xy(world_position)
            {
                // from actor translation find what sector and cell it is in
                let (tform, mut pathing) = actor_q.get_single_mut().unwrap();
                let (source_sector_id, source_field_cell) = map_dimensions
                    .get_sector_and_field_cell_from_xy(tform.translation.truncate())
                    .unwrap();
                // send an event asking for a path to be generated
                event.send(EventPathRequest::new(
                    source_sector_id,
                    source_field_cell,
                    target_sector_id,
                    goal_id,
                ));
                // update the actor pathing (we get the route later once it is built)
                pathing.source_sector = Some(source_sector_id);
                pathing.source_field_cell = Some(source_field_cell);
                pathing.target_sector = Some(target_sector_id);
                pathing.target_goal = Some(goal_id);
                pathing.portal_route = None;
            } else {
                warn!("Cursor out of bounds");
            }
        }
    }
}
```

The actor can then query the `RouteCache` to begin following a high-level portal-to-portal route.

Note this example is very basic as it only handles a single actor, in an application you'd devise your own handling system:

```rust
fn actor_update_route(mut actor_q: Query<&mut Pathing, With<Actor>>, route_q: Query<&RouteCache>) {
    let mut pathing = actor_q.get_single_mut().unwrap();
    // indicates whether the Actor has requested a route and doesn't have one assigned
    if pathing.target_goal.is_some() && pathing.portal_route.is_none() {
        // check the cache to see if the route has been built yet
        // routes are ID'ed by the starting and end sectors and the target field cell at the end
        let route_cache = route_q.get_single().unwrap();
        if let Some(route) = route_cache.get_route(
            pathing.source_sector.unwrap(),
            pathing.target_sector.unwrap(),
            pathing.target_goal.unwrap(),
        ) {
            // it has! So set the Actors (high level) pathing route and you
            // can optionally implement a pre-cursor steering pipeline to walk
            // the route until the FlowFields are built too
            pathing.portal_route = Some(route.clone());
        }
    }
}
```

And once the `FlowFields` have been built they can query the `FlowFieldCache` instead and apply/queue up some kind of movement.

Note this example is very basic as it only handles a single actor, in an application you'd devise your own handling system:

```rust
const ACTOR_SPEED: f32 = 64.0;
fn actor_steering(
    mut actor_q: Query<(&mut Transform, &mut Pathing), With<Actor>>,
    flow_cache_q: Query<(&FlowFieldCache, &MapDimensions)>,
) {
    let (mut tform, mut pathing) = actor_q.get_single_mut().unwrap();
    let (flow_cache, map_dimensions) = flow_cache_q.get_single().unwrap();

    if pathing.target_goal.is_some() {
        // lookup the overarching route
        if let Some(route) = pathing.portal_route.as_mut() {
            // find the current actors postion in grid space
            let (curr_actor_sector, curr_actor_field_cell) = map_dimensions
                .get_sector_and_field_cell_from_xy(tform.translation.truncate())
                .unwrap();
            // tirm the actor stored route as it makes progress
            // this ensures it doesn't use a previous goal from
            // a sector it has already been through when it needs
            // to pass through it again as part of a different
            // segment of the route
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
                        let dir = get_2d_direction_unit_vector_from_bits(cell_value);
                        let velocity = dir * ACTOR_SPEED;
                        // move the actor based on the velocity
                        tform.translation += velocity.extend(0.0);
                    }
                    break 'routes;
                }
            }
        }
    }
}
```

NB: generated FlowFields and Routes expire from their caches after 15 minutes, your steering pipeline may need to send a new `EventPathRequest` if one gets expired that an actor was relying on.

NB: when a CostField is modified Portals and the PortalGraph are updated and any Routes or FlowFields involving the modified Sector CostField are removed. This means an actor would need a way of knowing (implicitly or explicitly) that it needs to have a new Route made via an `EventPathRequest`. Hopefully auto regeneration of these routes can be solved to take the burden away from the actors, see [issue](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/issues/8).

# Features

* `serde` - enables serlialisation on some data types
* `ron` - enables reading `CostField` from files. NB: fixed-size arrays in `.ron` are written as tuples
* `csv` - enables creating all of the `CostFields` by reading from a directory of csv files. Note that csv filenames need to follow the sector ID convention of `column_row.csv`, the underscore is important, and the path of the directory should be fully qualified and the files themselves should not contain any headers
* `2d` - enables interface methods when working with Flowfields in a 2d world
* `3d` - enables interface methods when working with FlowFields in a 3d world
* `heightmap` - allows initialising the `CostField`s from a greyscale png/jpeg where each pixel of the image represents a `FieldCell`. Alpha channel is optional (it'll just be ignored if included in the image). A pixel with colour channels `(0, 0, 0, 255)` (black) represents an impassable `255` cost whereas `(255, 255, 255, 255)` (white) is translated as a cost of `1`, channel values in between will be more expensive costs

# Performance

Benchmarks are split into two categories:

* Data initialisation
  * [init_cost_fields](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/init_cost_fields.rs) - measures the time it takes to initalise 100x100 sector `CostFields`
  * [init_portals](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/init_portals.rs) - measures the time it takes to build `Portals` across 100x100 sectors
  * [init_portal_graph](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/init_portal_graph.rs) - measure the time it takes to build the `PortalGraph` for 100x100 sectors
  * [init_bundle](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/init_bundle.rs) - measure the total time it takes to have the `FlowFieldTilesBundle` ready
* Algorithm use - measures generating a set of FlowFields
  * [calc_route](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/calc_route.rs) - measures how long it takes to generate a route from one corner of a 100x100 sector layout to the opposite corner
  * [calc_flow_open](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/calc_flow_open.rs) - measure how long it takes to create a full set of `FlowFields` describing movement across uniform `CostFields` (cost = 1) from one corner to another
  * [calc_flow_maze](https://github.com/BlondeBurrito/bevy_flowfield_tiles_plugin/blob/main/benches/calc_flow_maze.rs) - measures how long it takes to create a full set of `FlowFields` describing movement from one corner to another in a 100x100 sector world. The world is composed of vertical corridors meaning that the actor has to path up and down to eventually snake it's way to the goal

Currently the slowest area is generating the `PortalGraph` (7s on my machine) so this should be some initialisation that happens behind the scenes (like a loading screen or some such).

Depending on pathing complexity I've seen `FlowField` generation range from 5-90ms.

# LICENSE

Dual license of MIT and Apache.
