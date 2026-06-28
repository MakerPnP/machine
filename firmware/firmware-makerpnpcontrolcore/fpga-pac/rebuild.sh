#!/usr/bin/env sh

chiptool generate --svd ../fpga/assets/svd/fpga.svd --output src
rustfmt src/lib.rs
#git apply < patches/0001-delete-interrupt-block.patch
#git apply < patches/0002-delete-rt-block.patch
#git apply < patches/0003-octospi-fifo-bypass.patch
rm src/device.x
