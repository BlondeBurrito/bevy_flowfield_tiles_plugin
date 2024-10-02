# Changelog

All notable changes to this project will be documented in this file.

## [0.11.0] - 2024-10-02

### Refactor

- [**breaking**] Moved LOS from flow layer to integration layer (#67)

The Line Of Sight (LOS) calculation has been rewritten and moved to the Integration layer. Previously the calculation would touch every single cell in the target sector but now it only touches viable cells by performing impassable cell corner checks. Additionally the previous integration cost calculation would touch every field cell but now it only touches cells which failed the LOS pass.

The Integration layer is refactored into three phases:

1. Portal expansion - a Portal represents the midpoint of a traversable sector boundary, when generating the field we expand the portals to cover their entire segment - this increases efficiency so that an actor can more directly approach its goal rather than zig-zagging to portal boundary points
2. LOS propagation - expands a wavefront from the goal identifying cells that have a clear line to the goal, this means that near the goal an actor can abandon reading from the FlowFields and follow a simple vector - this promotes the best quality pathing near the goal
3. Integrated cost calculation - from cells blocked off from LOS a series of wavefronts propagate across blocked off cells producing a cumulative cost (flow) which forms a gradient towards the goal

The Flow layer now simply reads off values from the Integration layer to produce the navigational FlowField.

The public interface to retrieve FlowFields `get_field()`, has a breaking change whereby the end goal sector ID is now an additional param - this change promotes more precise fields to actors as this extra ID forms part of the unique ID of a particular field so an actor chooses a field precisely rather than getting a similar one that was "kinda good enough" near the goal.

Some extras within this big refactor include: a new example showing LOS, improved examples to prevent actor tunnelling given that a physics simulation is used to help demonstrate the algorithm and perf improvements to the Portal Graph for edge weight calculation.


## [0.10.2] - 2024-09-02

### Miscellaneous Tasks

- Bump MSRV, fix clippy lint syntax




### Performance

- Enable wayland support for bevy  (#66)

prevents examples from lagging on wayland sessions. If not enabled bevy is built with x11 support only, and lags like crazy for no fault of the pathing algo, giving bad impression to the user

---------


### Refactor

- Migrate example physics to avian2d (#64)




## [0.10.1] - 2024-07-24

### Features

- Members of FlowFieldTilesBundle are public, make costfield scaling function public (#62)

- by making these items public users are able to more easy write custom ways of initialising the bundle


## [0.10.0] - 2024-07-04

### Features

- Enable setting internal and external cell costs when using 2d meshes (#60)



- Updated to bevy 0.14 (#61)




## [0.9.0] - 2024-06-16

### Documentation

- FlowfieldTilesBundle -> FlowFieldTilesBundle fix in readme (#56)




### Features

- Initialise Flowfields from a list of 2d meshes (#59)

A vector of 2d Bevy meshes can be used to initialise the Flowfields where any cells within the boundaries of a mesh are marked as having a pathing cost of 1. TriangleList and TriangleStrip topologies are both supported. 

Note that if two meshes overlap/touch then the boundaries of those meshes will be marked as impassable. The aim in a future update is to treat overlapping/touching boundaries as continuous pathable space - this update helps lay the groundwork for being able to update Flowfields during runtime with user-spawned meshes which will bring a more high-level API to make the way in which a user interacts with this plugin more friendly. Once this 2d functionality is complete a 3d variant will be introduced


### Miscellaneous Tasks

- Clippy and remove unused code (#58)




### Performance

- Init benchmarks with heightmaps (#55)




### Testing

- Enable sparse benchmark




## [0.8.0] - 2024-02-27

### Bug Fixes

- Fix code-coverage action (#53)



- [**breaking**] Various related fixes to CostField mutation (#54)

Fixed example border colliders

Improved perf of route recalculation

Fixed bug for LOS calculation in corners

Reduced overhead of Flowfield visualisation example

Added example `2d_mutate_costfields` to showcase modifying CostFields at runtime and new FlowFields being calculated for pathfinding actors to follow

Example `visualise_portals` supports flipping FieldCell costs between `1` and `255` to showcase Portals being recalculated

Improved example readability

Aligned `2d` and `3d` method names

PortalGraph split into 3 distinct collections of nodes, internal_edges and external_edges

Fixed Node generation when two Nodes overlap in a sector corner as portals to different neighbouring sectors

Added a handler in examples for scenarios where an actor collides with a corner and gets bounced into a sector that's not part of its route - it'll clear its old route and request a new one

Added an example showcasing a variety of CostField values - can demonstrate an actor prioritising better costs to follow (but in the goal sector LOS check will override bad costs as the distance is considered so short it's best to path directly to the end)


## [0.7.0] - 2024-02-20

### Features

- Added initialising CostFields from a png/jpeg heightmap (greyscale image) (#51)

Added optional feature `heightmap` - allows initialising the `CostField`s from a greyscale png/jpeg where each pixel of the image represents a `FieldCell`. Alpha channel is optional (it'll just be ignored if included in the image). A pixel with colour channels `(0, 0, 0, 255)` (black) represents an impassable `255` cost whereas `(255, 255, 255, 255)` (white) is translated as a cost of `1`, channel values in between will be more expensive costs

- Update to bevy 0.13 (#52)




## [0.6.1] - 2024-02-07

### Bug Fixes

- Warn if a portal is explored that doesn't have any egdes




## [0.6.0] - 2024-02-06

### Documentation

- Clarify sector_resolution doc comment




### Features

- Bump MSRV to 1.74



- [**breaking**] On CostField mutate purge outdated routes and request new ones (#46)



- `PortalGraph` supports reflect (#48)




### Miscellaneous Tasks

- Adopt lints in Cargo manifest (#44)



- Readme



- Added generated flowfield counter to 2d_continuous (#50)




### Performance

- [**breaking**] Improved field calculations and route handling (#45)

Integration and flow fields are built in stages, this avoids blocking the main thread for too long, observed average FPS increased from 70 to 165 with the `2d_continuous` example, while number of simultaneous pathfinding actors increased from 600 to 1600

Replaced flowfield blocked diagonal `len()` check with a `bool` check (gets rid of pointless `vec` heap allocations)

`PortalGraph` completely rewritten, crate `petgraph` is no longer a dependency of this library (reduced memory usage as no translator from Node/Edge Index to Sector/FieldCell required)

Events to request a pathfinding route are now filtered to avoid processing duplicates (note: stable FPS at 165, used to jump between 150 and 200 wildly)


### Testing

- Improved test coverage to ~90% (#47)



- Replaced collision detection in 2d examples (#49)

Previously the 2d examples used my janky collision detection which allowed some actors to tunnel into colliders and get stuck. That's all been removed and now the `bevy_xpbd_2d` crate has been added as a dev-dependency to showcase the algorithm working alongside a physics simulation. So far `xpbd` has been really promising and easy to use and now the 2d examples flow nicely


## [0.5.1] - 2023-11-24

### Bug Fixes

- Added ratio in feature 3d to convert Vec3 into sector and field (column, row) (#43)




## [0.5.0] - 2023-11-23

### Features

- Updating to bevy 0.12 and prepare 0.5.0 release (#40)




## [0.4.0] - 2023-09-12

### Features

- [**breaking**] Scale impassable cells to close gaps too small for large actor pathing (#32)

Added actor_scale factor to grow the baseline CostFields so that a very wide actor cannot retrieve any FlowFields which would guide them through a narrow gap between 255 impassable cells that's smaller than they are

- Impl line of sight bit flag (#33)

Added a line of sight calculation in the target sector of a PathRequest marking cells that have an unobstructed direct path to the goal. For any FieldCell where the LOS bit flag has been toggled an actor can abandon following the FlowField and move in a direct vector to the goal


### Refactor

- 2d examples make use of colliders and increased timestep (#31)

2d examples have a primitive collider system - this allows for an increased timestep to be used to control actor movement and allowing for more fluid examples


## [0.3.0] - 2023-08-27

### Bug Fixes

- Graph updates should no longer collide due to multiple cost changes (#30)




### Features

- Example of continuous actor spawning




### Miscellaneous Tasks

- Enable feature docs on docrs




## [0.2.0] - 2023-07-31

### Documentation

- Updated usage section to align with cleaner interface (#27)



- Explain each bench (#29)




### Features

- [**breaking**] Added reflection (#25)

* Reflection on `Ordinal`, `MapDimensions`, `CostField`, `Portals`, `FlowField`, `SectorID`, `FieldCell`, `RouteMetadata` and `FlowFieldMetadata`


### Refactor

- [**breaking**] Replace sector and field tuple IDs with data structures of 'SectorID' and 'FieldCell' (#22)



- [**breaking**] Sector user interface embedded into MapDimension data (#24)

* Sector interface functions are members of `MapDimensions`
  * Methods are gated behind features `2d` and `3d`
* Sector resolution no longer `const`, set at bundle creation allowing for a customised scale factor and very granular fields
  * A `30x30` world with a resolution of `10` will produce a `3x3` sector representation, where a `FieldCell` represents a `1x1` unit area
  * A `30x30` world with a resolution of `3` will produce a `10x10` sector representation, where a `FieldCell` represents a `0.3x0.3` unit area
* Renamed bundle `new_from_disk()` method to `from_ron()` to align with serialising csv 
* Renamed `SectorCostField::from_file()` to `SectorCostField::from_ron()`
* Renamed `CostField::from_file()` to `CostField::from_ron()`


## [0.1.0] - 2023-07-24

### Bug Fixes

- 3d coord conversion to grid space



- Route filtering allows routes back to starting sector



- Examples that hook in use new route filtering



- Improve example steering when pathing needs repeating in a sector



- Fix generation of the maze benchmark csv files




### Documentation

- Tidy/add module doc comments




### Features

- Mark cached routes and fields as dirty when costfields change



- Csv to sector costfields support



- Example visualise portals



- Prep 0.1




### Miscellaneous Tasks

- Tidy up warnings



- Allow publishing




### Performance

- Added initial benchmarks




### Refactor

- Portal graph rewritten, int visualisation working



- Caches use struct based keys



- Asset path structure




<!-- generated by git-cliff -->
