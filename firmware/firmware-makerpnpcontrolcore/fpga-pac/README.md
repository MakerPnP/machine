# FPGA-PAC

Peripheral Access Crate for the FPGA.

## Building

```
cargo install --git https://github.com/embassy-rs/chiptool --locked
```

Currently this requires chiptool 1.0.0. YMMV with other versions.

## Generating

The process is:
* use chiptool to generate rust source from the SVD file in the FPGA folder (from this repo).
* format the source code using rustfmt.
* apply patches to the generated code.

```
./rebuild.sh
```

There are a set of patches in the `patches` directory that are applied to the generated code.
The patches themselves are generated from commits to this repository in such a way that the generated and patched code
results in the same code as the repository.
Patches are creating using `git format-patch` and then added to the `patches` directory and the `rebuild.sh` script
is updated to include them.

## Using

* This crate is added as a dependency to the firmware crate.
* the FPGA is placed in memory mapped mode.
* the developer uses the FPGA as if it was a regular STM32 peripheral using the same register API pattern as the MCU.

### Examples

#### Turning on an FPGA controlled LED

```rust
fpga_pac::LED.led_ctrl().modify(|w| {
    w.set_mcu_led(true);
});
```

#### Configuring WS2812 LEDs

```rust
fpga_pac::ws2812_0.ws_ctrl().modify(|w| {
    w.set_enabled(true);
    w.set_mode(self.color_ordering.into());
});
fpga_pac::ws2812_0.ws_tx_config().write(|w| {
    w.set_leds_count(self.led_count);
});
```

In the latter example, modifying 2 register fields result in a read from the first register, followed by a single 
OctoSPI transaction that writes to 2 adjacent registers.

## Updating patches

This is currently a manual process.

* rebase on a commit before the patches were applied
* COPY patch commits to the end of commit history
* add a break before the commits are applied
* disable patches in rebuild.sh, commit
* rebuild, commit unpatched svd.
* continue rebasing, updating patch commits, resolve merge conflicts if they don't apply cleanly to the generated code. do not modify the patch files themselves.
* run git format-patch HEAD~n for the patch commits
* move the updated patch files to the patches directory.
* run git add on the updated patch files.
* run git rm on any unused patch files.
* add the patches back to rebuild.sh, run rebuild.sh, when it applies cleanly commit updated patches and rebuild.sh as one commit.
* if any new patches are required, create commits that modify the generated code. then as a separate commit, use format-patch again and add them to the repo.
* rebase if required to clean up the history.
