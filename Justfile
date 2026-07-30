test: lint build unit-test

unit-test: build-dev
    cargo test
    bats tests.bats

fuzzy-test: build-dev
    bash fuzzy_test.sh > fuzzy_test.log
    tail -n 8 fuzzy_test.log

benchmark: install
    hyperfine \
        "dc -e '10000 [d1-d1<F*]dsFxp'" \
        "rdc '10000 [d1-d1<F*]dsFxp'"

build:
    cargo build --release
    cp target/release/rdc .

build-dev:
    cargo build --profile dev

lint:
    cargo fmt
    cargo clippy

install: lint
    cargo install --path .

repl:
    cargo run --

compare cmd: build-dev
    #!/bin/bash
    set -e
    echo "dc -e '{{ cmd }}'               # => $(dc -e '{{ cmd }}')"
    echo "./target/debug/rdc '{{ cmd }}'  # => $(./target/debug/rdc '{{ cmd }}')"

clean:
    rm -rf ./target
    rm -rf ./rdc
