# MakerPnPControl-CORE FPGA firmware

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

The firmware doesn't do much yet, very early days. This document is in flux and there may be errors, refer to the
source in case of any discrepancies.

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

### QuadSPI MCU interface

There is a simple QuadSPI interface which allows an MCU to read/write registers from/to the FPGA using a simple
command set.

* Continuous read is supported.
* The command must be sent for every access.
* CS must be driven low before the first clock cycle
* Clock polarity LOW, changes are latched on low to high transitions.


#### Commands

| Command | Name     | Address Length            | Dummy/Control          | Data     |
|--------:|----------|---------------------------|------------------------|----------|
| 0x10    | READ_16  | 2 bytes (16 bit address)  | 4 bytes/8 clock cycles | Data...  |
| 0x90    | WRITE_16 | 2 bytes (16 bit address)  | None                   | Data...  |

For all commands the leading bit of the command is used to indicate read or write mode.
MSB = 0 == READ, MSB = 1 = WRITE.

Byte-wise:

| Command | Name     | B1      | B2            | B3           | B4      | B5      | B6      | B7      | B8      | B9      | B10     | B11     | B12     | Bxx       |
|--------:|----------|---------|---------------|--------------|---------|---------|---------|---------|---------|---------|---------|---------|---------|-----------|
|    0x10 | READ_16  | Command | Address[15:8] | Address[7:0] | Dummy   | Dummy   | Dummy   | Dummy   | Data[0] | Data[1] | Data[2] | Data[3] | Data[4] | Data[...] |
|    0x90 | WRITE_16 | Command | Address[15:8] | Address[7:0] | Data[0] | Data[1] | Data[2] | Data[3] | Data[4] | Data[5] | Data[6] | Data[7] | Data[8] | Data[...] |

Bit wise: 

READ_16 Timing Example (4-bit IO)

| IO Line | C1        | C2        | C3        | C4        | C5        | Dummy (C6–C13, 8 cycles) | C14   | C15   | C16   | C17   |
|--------|-----------|-----------|-----------|-----------|-----------|--------------------------|-------|-------|-------|-------|
| IO[3]  | CMD[3]    | ADDR[15]  | ADDR[11]  | ADDR[7]   | ADDR[3]   | 0                        | D0[7] | D0[3] | D1[7] | D1[3] |
| IO[2]  | CMD[2]    | ADDR[14]  | ADDR[10]  | ADDR[6]   | ADDR[2]   | 0                        | D0[6] | D0[2] | D1[6] | D1[2] |
| IO[1]  | CMD[1]    | ADDR[13]  | ADDR[9]   | ADDR[5]   | ADDR[1]   | 0                        | D0[5] | D0[1] | D1[5] | D1[1] |
| IO[0]  | CMD[0]    | ADDR[12]  | ADDR[8]   | ADDR[4]   | ADDR[0]   | 0                        | D0[4] | D0[0] | D1[4] | D1[0] |

#### Register Map

| Address | R/W | Name          | Length (bytes) | Purpose                |
|--------:|:---:|---------------|----------------|------------------------|
|    0x00 | RO  | IDENT         | 4              | A fixed identifier     |
|    0x04 | RO  | VERSION       | 4              | A fixed version number |
|    0x10 | WO  | CONFIG_1      | 4              | Config register 1      |
|    0x20 | RW  | LED           | 1              | Read/Write LED status  |
|    0x24 | RO  | IO_IN_1       | 1              | Read IO inputs         |
|     ... |     | RESERVED      |                |                        |
|    0x40 | RW  | ENCODER_1     | 4              | encoder 1              |
|    0x44 | RW  | ENCODER_2     | 4              | encoder 2              |
|    0x48 | RW  | ENCODER_3     | 4              | encoder 3              |
|    0x4C | RW  | ENCODER_4     | 4              | encoder 4              |
|    0x50 | RW  | ENCODER_5     | 4              | encoder 5              |
|    0x54 | RW  | ENCODER_6     | 4              | encoder 6              |
|     ... |     | RESERVED      |                |                        |
|   0x1FC | RO  | END_OF_MEMORY | 4              |                        |

REGISTERS - MEMORY MAPPED PERIPHERAL MAP (32-bit REGISTERS)
=====================================================================

All registers are 32-bit wide unless otherwise specified.
Big-endian field ordering is shown for multi-byte decompositions.

##### 0x00 - IDENT (RO)

```
31-0
+------------------------------------------+
|                IDENT                     |
+------------------------------------------+
|              0xFACEB00B                  |
+------------------------------------------+
```
Type: Read-Only
Reset Value: 0xFACEB00B
Description: Fixed hardware identifier


##### 0x04 - VERSION (RO)

```
31-24     23-16      15-8       7-0
+------------------------------------------+
| MAJOR   | MINOR    | PATCH    | BUILD    |
+------------------------------------------+
```

Field mapping:
- [31:24] MAJOR (u8)
- [23:16] MINOR (u8)
- [15:8]  PATCH (u8)
- [7:0]   BUILD (u8)

Type: Read-Only
Description: Firmware/hardware version encoding


##### 0x10 - CONFIG_1 (WO)

```
31-1                               0       
+------------------------------------------+
|              RESERVED            | RESET |
+------------------------------------------+
```
Bit definitions:
- [0]   RESET (1 = reset system, self-clears)
- [31:1] RESERVED (ignored)

Type: Write-Only
Description: Configuration register


##### 0x20 - LED (RW)

```
7-2                   1         0         
+------------------------------------------+
|          RESERVED   | MCU_ACT | FPGA_ACT |
+------------------------------------------+
```

Bit definitions:
- [0]   FPGA_ACT LED control
- [1]   MCU_ACT LED control
- [7:2] RESERVED (must be written as 0)

Type: Read/Write
Reset Value: 0b0000_0001
Description: LED control register


##### 0x24 - IO_IN_1 (RO)

```
7-2                        1       0
+------------------------------------------+
|               RESERVED   | USER1 | USER0 |
+------------------------------------------+
```

Bit definitions:
- [0] BUTTON_USER_0
- [1] BUTTON_USER_1
- [7:2] RESERVED

Type: Read-Only
Description: Digital input status register


##### 0x40 - 0x54 - ENCODER_[1-6] (RW)

```
31-0
+------------------------------------------+
|               ENCODER VALUE              |
+------------------------------------------+
```
Type: Read/Write
Description: 32-bit encoder counter/value register

##### 0x1FC - END_OF_MEMORY (RO)

```
31-0
+------------------------------------------+
|              0xDEADCODE                  |
+------------------------------------------+
```

Type: Marker
Description: End-of-memory sentinel value
