# repo_template

Template of a Rust repo which includes some helper files for configuring tools I find useful for development.

## justfile

The [just](https://github.com/casey/just) command line runner is very useful for running a series of build steps/commands locally.

In particular I like to use it to run a debug build (so the compiler can tell me about overflow errors and things), run all tests, generate documentation, compile the binary and finally run it - all from typing `just r` in a terminal.

## Diagrams

Under `./docs` are a series of puml diagrams.

To generate a diagram use `just` with `just diagram [diagram_name]`, or to generate all of them `just diagrams`.

## rustfmt.toml

Controls formatting settings. I have a prefernce for using tabs simply because in shared projects individuals have their own preference for indentation depth and so automatic tab resizing can make a code base gentler on the eyes.

## clippy.toml

Currently commented out, as I use clippy more I suspect to customise what it does.

## cliff.toml

[git-cliff](https://github.com/orhun/git-cliff) is a very cool changelog generator which uses the style of [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). To generate a changelog based on what the next tag will be you can run `git cliff --tag v1.0.0 --output CHANGELOG.md`

## flamegraph

```sh
cargo install blondie
# set env:DTRACE to blondie_trace.exe
cargo install flmaegraph
# build the app
cargo build --profile=flamegraph
cargo build
cargo build --release
# then use admin terminal!!!
$env:BEVY_ASSET_ROOT="C:\source\rust\repo_template"
cargo flamegraph --package=repo_template --profile=flamegraph # release mode without stripping
cargo flamegraph --package=repo_template --dev # dev mode
```

## LICENSE

Dual license of MIT and Apache allowing a user to pick whichever they prefer for open source projects.
