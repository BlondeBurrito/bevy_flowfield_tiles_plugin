# TODO

- rename ordinal? cardinal? CompassPoints => contains cardinals and intercardinals (ordinals)
- scaling cost fields should be radial? instead of stepping in a dir n steps. Or should it be do a step, then analyse 1 radius ring around cell for walls?. Or should it be radius but don't mark whole ring as walls - when hitting a wall draw a straight line back to source for wall marking
- add a debug plugin to draw fields, use an auto_insert parameter to add a component to FlowFieldTiles which enables displaying them. When using multiple entities for actor sizes note to users that auto_insert should be false and they should manually add the component for trigger visibility of particular entity fields
- currently updated costfields lead to out of date flowfields being deleted but not re-queued for replacement, how to address this?
- verify behaviour when Pathing pollable is replaced/consumed and when costfield updates are consumed, are tasks being properly cleaned up?
- 2d_continuous_mutate, actors get orphaned, look at tick counting when flow not found for too long
- 3d model doesnt match ron costfield layout!
- internal docs
- add tests for xy/xyz when origin is none zero
