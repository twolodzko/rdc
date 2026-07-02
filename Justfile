test: lint build
    bats tests.bats

build:
    cargo build --release
    cp target/release/rdc .

lint:
    cargo fmt
    cargo clippy

install: lint
    cargo install --path .

repl:
    cargo run --

clean:
    rm -rf ./target
    rm -rf ./rdc
