#!/bin/sh

echo "TZ: $TZ"
export START_TIME=`date -Iseconds`

echo "$START_TIME"
exec /app/webapp-rust-linux
