[![crates.io](https://img.shields.io/crates/v/bevy_flowfield_tiles_plugin)](https://crates.io/crates/bevy_flowfield_tiles_plugin)
[![docs.rs](https://docs.rs/bevy_flowfield_tiles_plugin/badge.svg)](https://docs.rs/bevy_flowfield_tiles_plugin)

# bevy_flowfield_tiles_plugin

Inspired by the work of [Elijah Emerson](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter23_Crowd_Pathfinding_and_Steering_Using_Flow_Field_Tiles.pdf) this is an attempt to implement the data structures and logic required to generate a Flowfield representation of a world which can be used to pathfind a moving actor.

| bevy | bevy_flowfield_tiles_plugin |
|------|-----------------------------|
| 0.11 |  0.1                        |

## Intro

Pathfinding in games can take different forms and those forms have certain benefits aligned with the type of game they are being applied to. Generally people run across:

* Way-point Graph - points in space linked together, very strict structure, an actor will move from one way-point to another. Great for games played on a small grid, can be cumbersome when multiple actors are sharing a path which could result in physics collisions
* NavMesh - a walkable surface generated from the topology of meshes in a game world. It allows for a range of dynamic movement within the confines of the mesh
* Flowfield Tiles - a means of handling crowd and flocking behaviour by generating a flow field (vector field) describing how an actor flows across a world. A large number of actors can flow in unison to an endpoint while sharing the same pathing data structure

For larger and larger environemnts with an increasing number of pathing actors it may be beneficial to adopt a flow field based approach. Flow fields are complex, it's effectively akin to fluid mechanics, so this is an attempt to bring an agnostic implementation to the [Bevy](https://github.com/bevyengine/bevy/tree/main) game engine.

### Definitions

* Sector - a slice of a game world composed of three 2D arrays called fields (Cost fields, Integration fields and Flow fields). A game world if effectively represented as a number of Sectors
* Cost fields - a 2D array describing how difficult it is to path through each cell of the array
* Cost - how difficult/expensive it is to path somewhere, you could also call it <i>weight</i>
* Portal - a navigatable point which links one Sector to another
* Integration fields
* Flow fields

# Process

To generate a navigation Flowfield the game world is divided into Sectors indexed by `(column, row)` and each Sector has 3 layers of data. Each layer aids the next in building out a path.

## Sector

For a 3-dimensional world the `x-z` plane defines the number of Sectors used to represent it with a constant called `SECTOR_RESOLUTION`, currently enforced at `10`. This means that a for a `30x30` world there would be `3x3` Sectors representing it. Each Sector has an associated unqiue ID taken as its position: `(column, row)`.

<img src="docs/sectors.png" alt="sectors" width="250"/>

Likewise for a `300x550` world you'll be looking at `55` columns and `30` rows. The advantage of dividing a world into Sectors (as opposed to treating the whole world as a giant Flowfield) is that the work in generating a path can be split into multiple operations and only touch certain sectors - thereby fewer field cells to number crunch.

## Cost Fields

A `CostFields` is an `MxN` 2D array of 8-bit values. The values indicate the `cost` of navigating through that cell of the grid. A value of `1` is the default and indicates the easiest `cost`, and a value of `255` is a special value used to indicate that the grid cell is impassable - this could be used to indicate a wall or obstacle. All other values from `2-254` represent increasing cost, for instance a slope or difficult terrain such as a marsh. The idea is that the pathfinding calculations will favour cells of `1` before any others.

<img src="docs/cost_fields.png" alt="cf" width="370"/>

This array is used to generate the `IntegrationFields` when requesting a navigatable path.

At runtime the `CostFields` are generated for each Sector with the default value. See the [Useage] section below for details on updating `CostFields` during an inital pass (i.e when loading a level) and tweaking it during gameplay for a world which dynamically evolves with obstacles (flipping a cell to `255`).

## Portals

Each Sector has up to 4 boundaries with neighbouring Sectors. Each boundary can contain Portals which indicate a navigatable point from the current Sector to a neighbour. Portals provide responsiveness - flow fields may take time to generate so when an actor needs to move a quick A* pathing query can produce an inital path route based on moving from one Portal to another. Once the flow fields have been built the actor can switch to using them instead.

For these sectors they are located away from any edges of the world which means each boundary can have Portals:

<img src="docs/portals.png" alt="portals" width="400" height="500"/><img src="docs/portals_adj.png" alt="portals" width="400" height="500"/>

A Portal is generated at the midpoint of a boundary - in situations where the `CostFields` contains `255` costs along the edge then multiple Portals may be generated at the midpoint of each valid pathable segment along the boundary and this is propagated to neighbouring Sectors so that every Portal has a neighbour buddy (as evident in the right hand Sector above, `S(1, 1)` portal `(9, 1)` allows movement into `S(2, 1)` portal `(0, 1)`).

On a larger scale (but still small) and for the simplist CostFields available, a `2x2` Sector grid produces predictable boundary Portals.

<img src="docs/sectors_portals.png" alt="sector_portals" width="400" height="400"/>

### Portal Graph

For finding a path from one Sector to another at a Portal level all Sectors and Portals are recorded within a graph known as `PortalGraph`. The [petgraph](https://github.com/petgraph/petgraph) library has been used as this store and it gets built in three stages:

1. For all Portals and Sectors add a graph `node`
2. For each sector create `edges` (pathable routes) to and from each Portal `node` - effectively create internal walkable routes of each sector
3. Create `edges` across the Portal `node` on all sector boundaries

This allows the graph to be queried with a `source` sector and a `target` sector and list of Portals are returned which can be pathed. When a `CostFields` is changed this triggers the regeneration of the sector Portals for the region that `CostFields` resides (and its neighbours to ensure homogenous boundaries) and the graph it updated with any new Portals `node`s and the old ones removed. This is a particularly dangerous and complicated area as the Sectors, Portals and fields are represented in 2D but the graph is effectively 1D - it's a bit long list of 'node's. To handle identifying a graph `node` from a Sector and field grid cell a special data field exists in `PortalGraph` nicknamed the "translator". It's a way of being able to convert between the graph data structure and the 2D data structure back and forth, so from a grid cell you can find its `node` and from a list of `node`s (like an A* result) you can find the location of each Portal.

## Integration Fields

## Flow Fields

# Useage

## Default

## Custom System Setup and Constraints

## Initialising Data

## Path Request

## Local Info/Tools

### justfile

The [just](https://github.com/casey/just) command line runner is very useful for running a series of build steps/commands locally.

In particular I like to use it to run a debug build (so the compiler can tell me about overflow errors and things), run all tests, generate documentation, compile the binary and finally run it - all from typing `just r` in a terminal.

### Diagrams

Under `./docs` are a series of puml (plantUML) diagrams.

To generate a diagram setup puml use `just` with `just diagram [diagram_name]`, or to generate all of them `just diagrams`.

### rustfmt.toml

Controls formatting settings. I have a prefernce for using tabs simply because in shared projects individuals have their own preference for indentation depth and so automatic tab resizing can make a code base gentler on the eyes.

### clippy.toml

Currently commented out, as I use clippy more I suspect to customise what it does.

### cliff.toml

[git-cliff](https://github.com/orhun/git-cliff) is a very cool changelog generator which uses the style of [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). To generate a changelog based on what the next tag will be you can run `git cliff --tag v1.0.0 --output CHANGELOG.md`

### flamegraph on windows

```sh
cargo install blondie
# set env:DTRACE to blondie_trace.exe
cargo install flmaegraph
# build the app
cargo build --profile=flamegraph
cargo build
cargo build --release
# then use admin terminal!!!
$env:BEVY_ASSET_ROOT="C:\source\rust\bevy_flowfield_tiles_plugin"
cargo flamegraph --package=bevy_flowfield_tiles_plugin --profile=flamegraph # release mode without stripping
cargo flamegraph --package=bevy_flowfield_tiles_plugin --dev # dev mode
```

## LICENSE

Dual license of MIT and Apache allowing a user to pick whichever they prefer for open source projects.
