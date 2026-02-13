#!/bin/sh
set -e
cargo build -r
cp ./target/release/mvc $HOME/.cargo/bin/
