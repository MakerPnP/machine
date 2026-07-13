// --------------------------------------------------------------------
// DEBUG LOGGING MACRO
// --------------------------------------------------------------------
`ifdef SIM
    `define DBG_LOG(args) $display args
`else
    `define DBG_LOG(args) // Evaluates to empty space in synthesis, removing Yosys overhead
`endif
