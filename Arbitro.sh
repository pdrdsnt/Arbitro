#!/bin/bash

npm run watch --prefix ./frontend &
PID1=$!

cargo run --manifest-path ./server/Cargo.toml &
PID2=$!

trap "kill $PID1 $PID2" SIGINT

wait -n

kill $PID1 $PID2 2>/dev/null


