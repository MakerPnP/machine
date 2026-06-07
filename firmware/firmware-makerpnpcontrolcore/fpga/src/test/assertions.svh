`ifndef ASSERTIONS_SVH
`define ASSERTIONS_SVH

package tb_assert;

  localparam string COLOR_RED   = "\033[31m";
  localparam string COLOR_GREEN = "\033[32m";
  localparam string COLOR_WHITE = "\033[0m";

  int unsigned assert_count = 0;
  int unsigned fail_count   = 0;

  task automatic report();

    string color;

    $display("================================");
    $display("ASSERTIONS: %0d", assert_count);

    if (fail_count != 0)
      color = COLOR_RED;
    else
      color = COLOR_GREEN;

    $display("%sFAILURES  : %0d%s", color, fail_count, COLOR_WHITE);
    $display("\033[37m");
    $display("================================");
    if (fail_count > 0) $stop;
  endtask

endpackage

import tb_assert::*;

// Updated Macro with 4 arguments (2 mandatory, 2 optional)
// FMT defaults to "%0d" for backward compatibility
// MSG defaults to "" (an empty string)
`define ASSERT_EQ(A, B, FMT="%0d", MSG="") \
  assert ((A) == (B)) assert_count++; \
  else begin \
    fail_count++; \
    if (MSG == "") begin \
      $error($sformatf("%sAssertion failed: %s == %s, actual = %s%s", \
                       COLOR_RED, `"A`", FMT, FMT, COLOR_WHITE), (B), (A)); \
    end else begin \
      $error($sformatf("%sAssertion failed: %s == %s, actual = %s - %s%s", \
                       COLOR_RED, `"A`", FMT, FMT, MSG, COLOR_WHITE), (B), (A)); \
    end \
  end
`endif