# MakerPnpControl-CORE
# yosys+nextpnr

YOSYS_ROOT := /usr/local/share/yosys
FLASH_ALGO_ROOT := ../../../../../dev-tools/makerpnpcore-w25q16jv

PROJECT  := core
TOP      := core_top
FAMILY   := ice40
TYPE     := hx8k
PACKAGE  := ct256

BUILD := build
SRC_MAIN := src/main
SRC_TEST := src/test

VERILOGS := $(shell find $(SRC_MAIN) -type f -name "*.v" ! -name "*_tb.v")
TB_FILES := $(shell find $(SRC_TEST) -type f -name "*_tb.v" ! -name "int_*.tb.v")
TB_INT_FILES := $(shell find $(SRC_TEST) -type f -name "*_tb.v" ! -name "int_*_tb.v")

TB := $(basename $(notdir $(TB_FILES)))
TB := $(patsubst %_tb,%,$(TB))

TB_INT := $(basename $(notdir $(TB_INT_FILES)))
TB_INT := $(patsubst %_tb,%,$(TB_INT))

COMMON_SRCS := $(SRC_MAIN)/common.v $(YOSYS_ROOT)/ice40/cells_sim.v

# ----------------------------
# TESTBENCH SOURCE DEFINITIONS
# ----------------------------

# Regular TBs
EXTRA_SRCS_blink :=
EXTRA_SRCS_wake :=
EXTRA_SRCS_timer_mux :=
EXTRA_SRCS_clock_out :=
EXTRA_SRCS_quadspi := $(SRC_MAIN)/registers/memory.v
EXTRA_SRCS_leds :=

# Special / integration TBs
EXTRA_SRCS_int_leds_mem := $(SRC_MAIN)/io/leds.v $(SRC_MAIN)/registers/memory.v

# Find RTL file matching module name
define FIND_RTL
$(shell find $(SRC_MAIN) -name "$1.v")
endef

# Find TB file
define FIND_TB
$(shell find $(SRC_TEST) -name "$1_tb.v")
endef

define TB_SRCS
$(call FIND_RTL,$1) \
$(call FIND_TB,$1) \
$(COMMON_SRCS) \
$(EXTRA_SRCS_$1)
endef

define TB_INT_SRCS
$(call FIND_TB,$1) \
$(COMMON_SRCS) \
$(EXTRA_SRCS_$1)
endef

SIM_TARGETS := $(addprefix sim_,$(TB)) $(addprefix sim_int_,$(INT_TB))

VVP_FILES := $(addprefix $(BUILD)/sim/,$(addsuffix _tb.vvp,$(TB))) $(addprefix $(BUILD)/sim_int/int_,$(addsuffix _tb.vvp,$(INT_TB)))
VCD_FILES := $(addprefix $(BUILD)/sim/,$(addsuffix _tb.vcd,$(TB))) $(addprefix $(BUILD)/sim_int/int_,$(addsuffix _tb.vcd,$(INT_TB)))

all: \
	$(PROJECT).bin \
	sim \

# -------------------------
# SYNTHESIS (FPGA build)
# -------------------------
$(BUILD):
	mkdir -p $(BUILD)

$(BUILD)/$(PROJECT).json: $(VERILOGS) $(BUILD) Makefile
	yosys -q -l $(BUILD)/yosys.log -p 'synth_${FAMILY} -top $(TOP) -json $(BUILD)/$(PROJECT).json' $(VERILOGS)

$(PROJECT).asc: $(BUILD)/$(PROJECT).json pins.pcf
	nextpnr-${FAMILY} -q -l $(BUILD)/nextpnr.log --${TYPE} --package ${PACKAGE} --json $(BUILD)/$(PROJECT).json --pcf pins.pcf --asc $(BUILD)/$(PROJECT).asc
	@echo ""
	@grep -B 1 "%$$" $(BUILD)/nextpnr.log
	@echo ""

$(PROJECT).bin: $(PROJECT).asc
	icepack $(BUILD)/$(PROJECT).asc $(BUILD)/$(PROJECT).bin


# -------------------------
# SIMULATION
# -------------------------

sim_%:
	mkdir -p $(BUILD)/sim
	iverilog -g2012 -o $(BUILD)/sim/$*_tb.vvp $(call TB_SRCS,$*)
	cd $(BUILD)/sim && vvp -n $*_tb.vvp
	@echo ""

sim_int_%:
	mkdir -p $(BUILD)/sim_int
	iverilog -g2012 -o $(BUILD)/sim_int/int_$*_tb.vvp $(call TB_INT_SRCS,int_$*)
	cd $(BUILD)/sim_int && vvp -n int_$*_tb.vvp
	@echo ""

wave_gtk_%: sim_%
	gtkwave $(BUILD)/$*.vcd

wave_%: sim_%
	surfer $(BUILD)/$*.vcd

# Optional: run all simulations
sim: $(SIM_TARGETS)

# -------------------------
# CLEAN
# -------------------------

clean:
	rm -f $(BUILD)/$(PROJECT).bin $(BUILD)/$(PROJECT).asc $(BUILD)/$(PROJECT).json \
		$(BUILD)/yosys.log $(BUILD)/nextpnr.log \
		$(VVP_FILES) $(VCD_FILES)
	@rmdir $(BUILD)/sim || true
	@rmdir $(BUILD)/sim_int || true
	@rmdir $(BUILD) || true
# -------------------------
# LOAD FPGA
# -------------------------

load: $(PROJECT).bin
	 probe-rs download --chip makerpnpcore-w25q16jv --chip-description-path $(FLASH_ALGO_ROOT)/target/definition.yaml $(PROJECT).bin --binary-format=bin --base-address 0x70000000 --verify
