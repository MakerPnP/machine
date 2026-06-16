# FPGA-PAC

Peripheral Access Crate for the FPGA.

## Building

```
cargo install --git https://github.com/embassy-rs/chiptool --locked
```

## Generating

To be automated...

```
chiptool generate --svd ../fpga/assets/svd/fpga.svd --output src
rustfmt src/lib.rs
```
