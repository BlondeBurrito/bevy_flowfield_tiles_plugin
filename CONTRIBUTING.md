# Contributing

* If you'd like to add something then raise an issue, fork the repo and submit a PR

## Local Tools

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
# https://crates.io/crates/blondie
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
