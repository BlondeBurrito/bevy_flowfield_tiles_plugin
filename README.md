[![crates.io](https://img.shields.io/crates/v/bevy_flowfield_tiles_plugin)](https://crates.io/crates/bevy_flowfield_tiles_plugin)
[![docs.rs](https://docs.rs/bevy_flowfield_tiles_plugin/badge.svg)](https://docs.rs/bevy_flowfield_tiles_plugin)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/blondeburrito/bevy_flowfield_tiles_plugin#license)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/blondeburrito/bevy_flowfield_tiles_plugin/ci.yml)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/blondeburrito/bevy_flowfield_tiles_plugin/code-cov.yml?label=CodeCov>85%)

# bevy_flowfield_tiles_plugin

Inspired by the work of [Elijah Emerson](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter23_Crowd_Pathfinding_and_Steering_Using_Flow_Field_Tiles.pdf) this is an attempt to implement the data structures and logic required to generate a Flowfield representation of a world which can be used to pathfind movable actors.

| bevy | bevy_flowfield_tiles_plugin |
|------|-----------------------------|
| [commit](https://github.com/bevyengine/bevy/commit/e9312254d8906e9f491c4f3761659df0d179687f) |  main                        |
| 0.11 (unreleased) |  0.1 (unreleased) |

# Table of Contents

1. [Intro](#intro)
1. [Useful Definitions](#useful-definitions)
1. [Design/Process](#designprocess)
1. [Usage](#usage)
1. [Features](#features)
1. [License](#license)

## Intro

Pathfinding in games can take different forms and those forms have certain benefits aligned with the type of game they are being applied to. Generally people run across:

* Way-point Graph - points in space linked together, very strict structure, an actor will move from one way-point to another. Great for games played on a small grid where movement needs to be restricted to precise lines, will be cumbersome when multiple actors are sharing a path - particularly when actors have some kind of collision system in place
* NavMesh - a walkable surface generated from the topology of meshes in a game world defining a valid area of movement. It allows for a range of dynamic movement within the confines of the mesh and is a natural evolution of the Way-point Graph
* FlowField Tiles - a means of handling crowd and flocking behaviour by generating a flow field (vector field) describing how an actor flows across a world. A large number of actors can flow in unison to an endpoint while sharing the same pathing data structure - saving compute resources and time

For larger and larger environemnts with an increasing number of pathing actors it may be beneficial to adopt a flow field based approach due to the data sharing and formation/group like movement it promotes. FlowField Tiles are complex, it's effectively akin to fluid mechanics, so this is an attempt to bring an agnostic implementation to the [Bevy](https://github.com/bevyengine/bevy/tree/main) game engine. My motivation for this is that I recently implemented a Way-point Graph for a prototype. In order to provide 'ok' actor movement it had to be made from 16 million data points. To prevent an actor from occasionally zig-zagging across the graph the granularity had to be boosted to 80 million data points to create a 'lifelike' impression of movement. That was just silly so I began looking into the history of pathfinding whereupon I stumbled across FlowField Tiles and decided to try and implement it with my favourite langauge and engine.

## Useful Definitions

* Sector - a slice of a game world composed of three 2D arrays called fields (`CostField`, `IntegrationField` and `FlowField`). A game world is effectively represented by a number of Sectors
* CostField - a 2D array describing how difficult it is to path through each cell of the array. It is always present in system memory
* Cost - how difficult/expensive it is to path somewhere, you could also call it <i>weight</i>, each cell of `CostField` has one of these
* Portal - a navigatable point which links one Sector to another
* IntegrationField - a 2D array which uses the CostField to determine a cumulative cost of reaching the goal/endpoint (where you want to path to). This is an ephemeral field - it exists when required to calculate a `FlowField`
* FlowField - a 2D array built from the `IntegrationField` which decribes how an actor should move (flow) across the world
* FlowField Cache - a means of storing `FlowFields` allowing multiple actors to use and reuse them
* Ordinal - a direction based on traditional compass ordinals: N, NE, E, SE, S, SW, W, NW. Used for discovery of Sectors/field cells
* Grid cell - an element of a 2D array
* Goal - the target grid cell an actor needs to path to
* Portal goal - a target point within a sector that allows an actor to transition to another sector, thus brining it closer towards/to the goal

# Design/Process

<details>
<summary>Click to expand!</summary>

To generate a set of navigation `FlowFields` the game world is divided into Sectors indexed by `(column, row)` and each Sector has 3 layers of data: `[CostField, IntegrationField, Flowfield]`. Each layer aids the next in building out a path. A concept of `Portals` is used to connect Sectors together.

## Sector

<details>
<summary>Click to expand!</summary>

For a 3-dimensional world the `x-z` plane defines the number of Sectors used to represent it with a constant called `SECTOR_RESOLUTION`, currently enforced at `10`. This means that a for a `30x30` world there would be `3x3` Sectors representing it. Each Sector has an associated unqiue ID taken as its position: `(column, row)`.

<img src="docs/sectors.png" alt="sectors" width="250"/>

Likewise for a `300x550` world you'll be looking at `30` columns and `55` rows. The advantage of dividing a world into Sectors (as opposed to treating the whole world as a giant `Flowfield`) is that the work in generating a path can be split into multiple operations and only touch certain sectors. Say for the `300x550` world you do treat it as a single set of fields - when calculating a path you could potentially have to calculate the Flowfield values for `165,000` grid cells. Splitting it into sectors may mean that your path only takes you through 20 sectors, thereby only requiring `2,000` `Flowfield` grid cells to be calculated.

</details>

## CostField

<details>
<summary>Click to expand!</summary>

A `CostField` is an `MxN` 2D array of 8-bit values. The values indicate the `cost` of navigating through that cell of the grid. A value of `1` is the default and indicates the easiest `cost`, and a value of `255` is a special value used to indicate that the grid cell is impassable - this could be used to indicate a wall or obstacle. All other values from `2-254` represent increasing cost, for instance a slope or difficult terrain such as a marsh. The idea is that the pathfinding calculations will favour cells with a smaller value before any others.

<img src="docs/cost_field.png" alt="cf" width="370"/>

At runtime the `CostField` is generated for each Sector with the default value - although with the feature `ron` it is possible to load the fields from disk. See the [Usage](#usage) section below for details on updating the `CostFields` during an inital pass (i.e when loading a level) and tweaking it during gameplay for a world which dynamically evolves with obstacles (flipping a cell to to a higher cost or impassable `255`).

This array is used to generate the `IntegrationField` when requesting a navigatable path.

</details>

## Portals

<details>
<summary>Click to expand!</summary>

Each Sector has up to 4 boundaries with neighbouring Sectors (2 or 3 when the sector is in a corner or along the edge of the game world). Each boundary can contain Portals which indicate a navigatable point from the current Sector to a neighbour. Portals serve a dual purpose, one of which is to provide responsiveness - `FlowFields` may take time to generate so when an actor needs to move a quick A* pathing query can produce an inital path route based on moving from one Portal to another and they can start moving in the general direction to the goal/target/endpoint. Once the `FlowFields` have been built the actor can switch to using them for granular navigation instead.

The following sectors are located away from any edges of the world which means each boundary can have Portals (the purple cells):

<img src="docs/portals.png" alt="portals" width="400" height="500"/><img src="docs/portals_adj.png" alt="portals" width="400" height="500"/>

A Portal is generated at the midpoint of a boundary - in situations where the `CostField` contains `255` costs along the edge then multiple Portals may be generated at the midpoint of each valid pathable segment along the boundary and this is propagated to neighbouring Sectors so that every Portal has a neighbour buddy (as evident in the right hand Sector above, `S(1, 1)` portal `(9, 1)` allows movement into `S(2, 1)` portal `(0, 1)`, even though `S(2, 1)` has a whole boundary that appears completely pathable).

On a larger scale (but still small) and for the simplist `CostField` available, a `2x2` Sector grid produces predictable boundary Portals.

<img src="docs/sectors_portals.png" alt="sector_portals" width="400" height="400"/>

### Portal Graph

For finding a path from one Sector to another at a Portal level all Sector Portals are recorded within a data strucutre known as `PortalGraph`. The [petgraph](https://github.com/petgraph/petgraph) library has been used within this structure to store navigational points and paths between them and it gets built in three stages:

1. For all Portals add a graph `node`
2. For each sector create `edges` (pathable routes) to and from each Portal `node` - effectively create internal walkable routes of each sector
3. Create `edges` across the Portal `node` on all sector boundaries (walkable route from one sector to another)

This allows the graph to be queried with a `source` sector and a `target` sector and a list of Portals are returned which can be pathed. When a `CostField` is changed this triggers the regeneration of the sector Portals for the region that `CostField` resides in (and its neighbours to ensure homogenous boundaries) and the graph is updated with any new Portals `nodes` and the old ones are removed. This is a particularly difficult and complicated area as the Sectors, Portals and fields are represented in 2D arrays but the graph is effectively 1D - it's a big long list of `nodes`. To handle identifying a graph `node` from a Sector and field grid cell a special data field exists in `PortalGraph` nicknamed the "translator". It's a way of being able to convert between the graph data structure and the 2D data structure back and forth, so from a grid cell you can find its `node` and from a list of `nodes` (like an A* result) you can find the location of each Portal in the grids.

</details>

## IntegrationField

<details>
<summary>Click to expand!</summary>

An `IntegrationField` is an `MxN` 2D array of 16-bit values. It uses the `CostField` to produce a cumulative cost to reach the end goal/target. It's an ephemeral field, as in it gets built for a required sector and then consumed by the `FlowField` calculation.

When a new route needs to be processed the field values are set to `u16::MAX` and the grid cell containing the goal is set to `0`.

A series of passes are performed from the goal as an expanding wavefront calculating the field values:

1. The valid ordinal neighbours of the goal are determined (North, East, South, West - when not against a sector/world boundary)
2. For each ordinal grid cell lookup their `CostField` value
3. Add the `CostField` cost to the `IntegrationFields` cost of the current cell (at the beginning this is the goal int cost `0`)
4. Propagate to the next neighbours, find their ordinals and repeat adding their cost value to to the current cells integration cost to produce their cumulative integration cost, and repeat until the entire field is done

This produces a nice diamond-like pattern as the wave expands (the underlying `CostField` is set to `1` here):

<img src="docs/int_field_prop0.png" alt="ifp0" width="300" height="310"/><img src="docs/int_field_prop1.png" alt="ifp1" width="300" height="310"/>
<img src="docs/int_field_prop2.png" alt="ifp2" width="300" height="310"/><img src="docs/int_field_prop3.png" alt="ifp3" width="300" height="310"/>

Now a dimaond-like wave isn't exactly realistic in a world of dynamic movement so at some point it should be replaced, based on various articles out there it seems people adopt the [Eikonal equation](https://en.wikipedia.org/wiki/Eikonal_equation) to create a more spherical wave expanding over the grid space.

When it comes to `CostField` containing impassable markers, `255` as black boxes, they are ignored so the wave flows around those areas:

<img src="docs/int_field_prop_impassable.png" alt="ifpi" width="300" height="310"/>

And when your `CostField` is using a range of values to indicate different areas to traverse, such as a steep hill:

<img src="docs/cost_field_hill.png" alt="cfh" width="300" height="310"/><img src="docs/int_field_prop_hill.png" alt="ifph" width="300" height="310"/>

So this encourages the pathing algorithm around obstacles and expensive areas in your world!

This covers calculating the `IntegrationField` for a single sector containing the goal but of course the actor could be in a sector far away, this is where `Portals` come back into play.

From the `PortalGraph` we can get a path of `Portals` to guide the actor over several sectors to the desired sector, extending above the `IntegrationField` of the goal sector has been calculated so next we "hop" through the boundary `Portals` working backwards from the goal sector to the actor sector (Portals are denoted as a purple shade) to produce a series of `IntegrationFields` for the chaining Sectors describing the flow movement.

<img src="docs/int_field_sector_to_sector_0.png" alt="ifsts0" width="260" height="310"/><img src="docs/int_field_sector_to_sector_1.png" alt="ifsts1" width="260" height="310"/><img src="docs/int_field_sector_to_sector_2.png" alt="ifsts2" width="260" height="310"/>

In terms of pathfinding the actor will favour flowing "downhill". From the position of the actor and looking at its grid cell neighbours a smalller value in that sectors `IntegrationField` means a more favourable point for reaching the end goal, going from smaller to smaller values, basically a gradient flowing downhill to the destination.

This is the basis of a `FlowField`.

As an example for a `30x30` world, goal at `0` with an actor at `A`, an `IntegrationField` set interrogating all sector `Portals` may produce a set of fields looking similar to:

<img src="docs/int_field_prop_big_example.png" alt="ifpbe" width="75%"/>

Notice the cool waves that propagate out from the goal!

Generating the fields for this path programmatically leads to:

<img src="docs/generated_int_fields.png" alt="gif" width="75%"/>

Notice that we don't bother generating the fields for sectors the actor doesn't need to path through. Also a Portal represents the midpoint of a traversable sector boundary, when generating the field we expand the portal to cover its entire segment - this increases efficiency so that an actor can more directly approach its goal rather than zig-zagging to points.

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
* `0b0010_0000` - has line-of-sight to goal, an actor no longer needs to follow the field, it can move in a straight line to the goal. This avoids calculating field values that aren't actually needed (UNIMPLEMENTED)
* `0b0100_0000` - indicates the goal
* `0b1000_0000` - indicates a portal goal leading to the next sector

So a grid cell in the `FlowField` with a value of `0b0001_0110` means the actor should flow in the South-East direction. In terms of use don't worry about understanding these bit values too much, the [Usage](#usage) section shows the helpers for interpreting the values of the `FlowField` to steer an actor.

Using the `IntegrationFields` generated before, with an actor in the top right trying to reach the bottom left, we now generate the `FlowFields`:

<img src="docs/generated_flow_fields.png" alt="gff"/>

The thinner porition of each cell icon indicates the flow direction. The actor runs along the flow lines leading to the goal.

</details>

## Route & FlowField Cache

<details>
<summary>Click to expand!</summary>

To enable actors to reuse `FlowFields` (thus avoiding repeated calculations) all `FlowFields` get placed into a `FlowFieldCache` with an identification system.

The cache is made from two sister caches:

1. Route Cache - when an actor requests to go somewhere a high-level route is generated from describing the overall series of sector-portals to traverse (`PortalGraph` A*). If a `FlowField` hasn't yet been calculated then an actor can use the `route_cache` as a fallback to gain a generalist direction they should start moving in. Once the `FlowFields` have been built they can swap over to using those more granular paths. Additionally changes to `CostFields` can change portal positions and the real best path, `FlowFields` are regenerated for the relevant sectors that `CostFields` have modified so during the regeneration steps an actor can once again use the high-level route as the fallback
1. Field Cache

</details>

</details>
</br>

# Usage

```toml
[dependencies]
bevy_flowfield_tiles_plugin = "0.1"
```

## Default

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

## Initialising Data

```rust
	cmds.spawn(FlowfieldTilesBundle::new(map_length, map_depth));
```

## Path Request

## Actor Sizes

# Features

* `serde` - enables serlialisation on some data types
* `ron` - enables reading `CostField` from files. NB: fixed-size arrays in `.ron` are written as tuples

# Local Info/Tools

<details>
<summary>Click to expand!</summary>

## justfile

The [just](https://github.com/casey/just) command line runner is very useful for running a series of build steps/commands locally.

In particular I like to use it to run a debug build (so the compiler can tell me about overflow errors and things), run all tests, generate documentation and compile the release library - all from typing `just b` in a terminal.

## Diagrams

All diagrams should be generated from code or from screengrabs of `examples`

When using code based diagrams puml (plantUML) is the preferred tool.

To generate a diagram setup puml and use `just` with `just diagram [diagram_name]` to create the `.png`, or to generate all of them `just diagrams`.

## rustfmt.toml

Controls formatting settings. I have a prefernce for using tabs simply because in shared projects individuals have their own preference for indentation depth and so automatic tab resizing can make a code base gentler on the eyes.

## clippy.toml

Currently commented out, as I use clippy more I suspect to customise what it does.

## cliff.toml

[git-cliff](https://github.com/orhun/git-cliff) is a very cool changelog generator which uses the style of [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). To generate a changelog based on what the next tag will be you can run `git cliff --tag v1.0.0 --output CHANGELOG.md`

## flamegraph on windows

```sh
cargo install blondie
# set env:DTRACE to location and exe blondie_trace.exe
cargo install flmaegraph
# build the app
cargo build --profile=flamegraph
cargo build
cargo build --release
# then must use admin terminal!!!
$env:BEVY_ASSET_ROOT="C:\source\rust\bevy_flowfield_tiles_plugin"
cargo flamegraph --package=bevy_flowfield_tiles_plugin --profile=flamegraph # release mode without stripping
cargo flamegraph --package=bevy_flowfield_tiles_plugin --dev # dev mode
```

</details>

# LICENSE

Dual license of MIT and Apache.
