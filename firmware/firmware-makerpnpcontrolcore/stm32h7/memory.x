MEMORY
{
    FLASH    : ORIGIN = 0x08000000, LENGTH = 1024K
    RAM      : ORIGIN = 0x24000000, LENGTH = 320K
    RAM_D3   : ORIGIN = 0x38000000, LENGTH = 16K   /* SRAM4 */
    RAM_D2   : ORIGIN = 0x30000000, LENGTH = 32K   /* SRAM1 (16K)+SRAM2 (16K) */
    RAM_DTCM : ORIGIN = 0x20000000, LENGTH = 128K
    RAM_ITCM : ORIGIN = 0x00000000, LENGTH = 64K

    OCTOSPI1 (rw) : ORIGIN = 0x90000000, LENGTH = 256M /* For FPGA (first port) only */
    OCTOSPI2 (rw) : ORIGIN = 0x70000000, LENGTH = 256M /* For ESP-C6/FPGA (second port)/FLASH/EXT */
}

SECTIONS
{
    .ram_d3 :
    {
        *(.ram_d3)
    } > RAM_D3
}

SECTIONS
{
    .ram_d2 :
    {
        *(.ram_d2)
    } > RAM_D2
}

SECTIONS
{
    .octospi1 (NOLOAD) :
    {
      . = ALIGN(4);
      *(.octo1)
      *(.octo1*)
      . = ALIGN(4);
    } > OCTOSPI1
}