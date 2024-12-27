#! /bin/sh

cargo run -- \
  example/input.csv \
  example/output/output.csv \
  --fiducial FID2 \
  -f example/feeder_config.csv \
  -n example/nozzle_config.csv \
  -r example/packages.csv \
  -o 0:0,100:100 \
  -p 3:3:25:25 -e