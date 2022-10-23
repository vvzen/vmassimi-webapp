#!/bin/bash
cargo build --release

cp target/release/webapp-rust webapp-rust-macos
