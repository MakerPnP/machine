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

`define ASSERT_EQ(A, B) \
  assert ((A) == (B)) assert_count++; \
     else begin \
        fail_count++; \
        $error("%sAssertion failed: %s == %0d, actual = %0d%s", COLOR_RED, `"A`", (B), (A), COLOR_WHITE); \
    end \


`endif