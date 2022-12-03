#!/bin/bash
cargo build --release

cp target/release/webapp-rust data/webapp-rust-macos
