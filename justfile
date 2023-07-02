# very useful command line runner - https://github.com/casey/just
set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]
alias c := clippy
alias d := doc
alias db := debug
alias t := test
alias b := build
alias r := run
alias clog := changelog
# alias cn := clean

bt := '0'

export RUST_BACKTRACE := bt

# print recipes
default:
  just --list
# lint the code aggressively
clippy:
  cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::cargo_common_metadata -D clippy::missing_docs_in_private_items -D clippy::todo -W clippy::unimplemented
# run a chosen example
example NAME:
  cargo run --release --example {{NAME}} --all-features
# run benchmarks
bench:
  cargo bench -q --benches --workspace
# run a debug build so the compiler can call out overflow errors etc, rather than making assumptions
debug:
  cargo build --workspace
# run tests
test: debug
  cargo test --release --workspace --all-features
# generate documentation
doc:
  cargo doc --release --workspace --all-features
# build release bin/lib
build: test doc
  cargo build --release --workspace --all-features --package bevy_flowfield_tiles_plugin
# build and execute bin
run: build
  cargo run --release --package bevy_flowfield_tiles_plugin
# delete `target` directory
clean:
  cargo clean
# git push with a message and optional branch target
push MESSAGE +BRANCH='main':
  git add .
  git commit -m "{{MESSAGE}}"
  git push origin {{BRANCH}}
# generate a changelog with git-cliff-based on conventional commits
changelog TAG:
  git cliff --tag {{TAG}} --output CHANGELOG.md
# evaluate documentation coverage
doc-coverage:
  $env:RUSTDOCFLAGS="-Z unstable-options --show-coverage"
  cargo +nightly doc --workspace --all-features --no-deps --release
  # https://github.com/rust-lang/rust/issues/58154
code-coverage:
  cargo tarpaulin --release --workspace --all-features --include-tests --engine=llvm
# install the crate from the local source rather than remote
install:
  cargo install --path .
# Useful tools
dev-tools:
  cargo install loc;
  cargo install git-cliff;
  cargo install blondie;
  cargo install flamegraph;
  cargo install cargo-bloat;
  cargo install cargo-deadlinks;
  cargo install cargo-geiger;
  cargo install cargo-modules;
  cargo install --locked cargo-outdated;
  cargo install cargo-watch;
  cargo install hyperfine;
  cargo install rust-script;
  rust-script --install-file-association;
  cargo install --locked cargo-deny
  cargo install cargo-tarpaulin
# Generate a diagram from a puml ile under ./docs
diagram NAME:
  java -jar "C:\ProgramData\chocolatey\lib\plantuml\tools\plantuml.jar" docs/{{NAME}}.puml
# Generate all diagrams under ./docs
diagrams:
  ForEach ($i in Get-ChildItem -Path "./docs/*.puml") {java -jar "C:\ProgramData\chocolatey\lib\plantuml\tools\plantuml.jar" $i.FullName}