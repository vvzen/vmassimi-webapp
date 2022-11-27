#!/bin/bash

set -e

pushd ./services/webapp-rust
./build_for_linux.sh
popd

#docker-compose build --no-cache
docker-compose build
docker-compose up --remove-orphans
