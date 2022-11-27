#!/bin/bash
set -e
echo "-> Creating release build.."

export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc
cargo build --release --target=x86_64-unknown-linux-gnu

echo "-> Copying resulting binary.."
cp target/x86_64-unknown-linux-gnu/release/webapp-rust data/webapp-rust-linux

echo "-> All done."
