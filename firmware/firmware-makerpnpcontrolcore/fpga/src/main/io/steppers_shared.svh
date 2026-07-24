localparam DIR_NORMAL         = 1'b0;   // forward / increasing position
localparam DIR_REVERSE        = 1'b1;   // reverse / decreasing position

localparam PERIOD_DECREASING  = 1'b0;   // period shrinks each step (accelerating)
localparam PERIOD_INCREASING  = 1'b1;   // period grows each step (decelerating)

// 2-bit per-segment command, packed into CTST[25:24]
localparam CMD_RESERVED = 2'b00;
localparam CMD_MOVE           = 2'b01;  // finish segment, fall through to the
                                         // next queued segment with no pause
localparam CMD_MOVE_HALT      = 2'b10;  // finish segment, then stop and wait
                                         // for a fresh start strobe (end of profile)
localparam CMD_MOVE_HALT_WAIT = 2'b11;  // finish segment, then stop and wait
                                         // for a fresh start strobe (more
                                         // segments still queued)
