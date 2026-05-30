# MakerPnPControl-core FPGA firmware

This is the work-in-progress FPGA code for the ICE40HX8K-CT256 on the MakerPnPControl-CORE board.

Refer to the schematics here: https://github.com/MakerPnP/makerpnp-control-board/tree/master/PCBs/core

## Overview

The PCB has an STM32H735 MCU, ESP32-C6 and an ICE40HX FPGA.  There is a shared flash connected to all three
devices via OctoSPI (4-bit). The FPGA normally boots from flash. The MCU controls when the FPGA boots.

There are two user button on the CORE board.
There are two FPGA controllable LED outputs.


## Build Pre-requisites:

A working icestorm toolchain, using yosys, nextpnr-ice40, icepack, iverilog.

## Building

```
make
```

## Test benches

When you run `make` is will run all the simulations, output of each testbench can be viewed using a vcd viewer
such as Surfer (modern, built using Rust) or GTKWave (legacy)

## Flashing

Flashing the FPGA is done using the probe-rs flash algorithm for the MakePnPCore board which can be found here:

https://github.com/MakerPnP/dev-tools

The path to the tool is set in the `Makefile`.  The default path expects the dev-tools repo to be a checked out
as a sibling to this repo.

```
makerpnp
├── machine
└── dev-tools
```

Flash using this command

```
make load
```

Note: this will halt the MCU on the CORE board, you will need to reset/power-cycle the CORE board afterwards.

## Details

The firmware doesn't do much yet, very early days.

### pll / clock

The `pll.v` file is generated using this:  

```
icepll -i 50 -o 100 -m -f pll.v
```

Note that on the schematic H16 is used for TCXO and H16 is on BANK 1 and the ICETechnicalLibrary states:

> The SB_PLL40_CORE primitive should be used when the source clock of the PLL is driven by FPGA routing i.e. 
> when the PLL source clock originates on the FPGA or is driven by an input pad that is not in the bottom IO 
> bank (IO Bank 2).

and

> The SB_PLL40_PAD primitive should be used when the source clock of the PLL is driven by an input pad that is
> located in the bottom IO bank (IO Bank 2) or the top IO bank (IO Bank 0), and the source clock is not required
> inside the FPGA

so we do NOT use the `-p` argument for `icepll` which says:

> `-p Use SB_PLL40_PAD primitive instead of SB_PLL40_CORE`
