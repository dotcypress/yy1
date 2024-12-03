#! /bin/sh

cargo run -- \
  example/input.csv \
  example/output.csv \
  --fiducial FID2 \
  -f example/feeder_config.csv \
  -n example/nozzle_config.csv \
  -p 3:3:25:25 -e