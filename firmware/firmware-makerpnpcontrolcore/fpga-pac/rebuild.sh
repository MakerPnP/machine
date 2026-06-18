#!/usr/bin/env sh

chiptool generate --svd ../fpga/assets/svd/fpga.svd --output src
rustfmt src/lib.rs
