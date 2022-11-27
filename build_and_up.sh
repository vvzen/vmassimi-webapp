#!/bin/bash

set -e

pushd ./services/webapp-rust
./build_for_linux.sh
popd

docker-compose up --build
