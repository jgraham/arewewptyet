#!/bin/bash
set -ex

REL_DIR_NAME=$(dirname "$0")
SCRIPT_DIR=$(cd "$REL_DIR_NAME" && pwd -P)
cd "$SCRIPT_DIR"/..

cd build
cargo test

cd ../wptfyi
cargo test
