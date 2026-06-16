# MakerPnPControl-CORE Firmware

This is the firmware for the MakerPnpControl-CORE board.

The CORE board has 3 devices that need firmware:
1) STM32H735 MCU
2) ESP32C6 MCU
3) ICE40HX8K FPGA

Refer to the readme files in the corresponding subdirectories.

## Using the FPGA as to implement additional peripherals

In this firmware, the FPGA is used to implement additional peripherals.

The FPGA is connected to the H7 MCU via OctoSPI (4-bit parallel QuadSPI, 6 wires total). The FPGA implements a 
QuadSPI slave interface with commands for read and write, in big endian and little endian formats.

An SVD file is created for the FPGA, peripheral code genereration is done using 'chiptool' from the embassy project.

The STM32H7 OctoSPI1 peripheral is placed in memory-mapped mode, so that the MCU can read/write to the FPGA using the
little-endian read/write commands.

As an example, the buzzer can now be controlled by the MCU as if the MCU had a peripheral that implements the buzzer
functionality.

```rust
fpga_pac::BUZZER.buzzer_ctrl().modify(|w| {
    w.set_buzzer(true);
});
```

To add new peripherals, one implements peripheral registers in the FPGA as modules and adds the module to the
address decoder map.  Then the SVD is updated to include the new peripheral and rust-code is re-generated.  The resulting
peripheral can then be used by the MCU to control the new peripheral.

So if you've ever wished for new peripherals on your MCU, then this approach can be used as a working example.

### Peripherals currently implemented in the FPGA

- Standard single-color LED control (gpio)
- Buzzer
- Button inputs
- WS2812 RGB LED control, 2 channels, 256 up to 256 leds per channel.
- 6 Channel quadrature encoder support; with ABZ signals.
