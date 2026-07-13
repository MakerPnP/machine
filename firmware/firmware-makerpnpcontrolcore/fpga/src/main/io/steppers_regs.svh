localparam REG_STEP_CTRL = 8'h00;
localparam REG_STEP_TX_CONFIG = 8'h04;
localparam REG_STEP_SEG_CTST = 8'h10;
localparam REG_STEP_SEG_SPDM = 8'h14;

localparam REG_STEP_STATUS_0 = 8'h20;
localparam REG_STEP_STATUS_1 = 8'h24;
localparam REG_STEP_STATUS_2 = 8'h28;
localparam REG_STEP_STATUS_3 = 8'h2c;
localparam REG_STEP_STATUS_4 = 8'h30;
localparam REG_STEP_STATUS_5 = 8'h34;
localparam REG_STEP_STATUS_6 = 8'h38;
localparam REG_STEP_STATUS_7 = 8'h3c;

// Current absolute position (signed, in steps) per motor - a plain 32-bit
// two's-complement value read back in one cycle, same as the status
// registers.
localparam REG_STEP_POS_0 = 8'h40;
localparam REG_STEP_POS_1 = 8'h44;
localparam REG_STEP_POS_2 = 8'h48;
localparam REG_STEP_POS_3 = 8'h4c;
localparam REG_STEP_POS_4 = 8'h50;
localparam REG_STEP_POS_5 = 8'h54;
localparam REG_STEP_POS_6 = 8'h58;
localparam REG_STEP_POS_7 = 8'h5c;