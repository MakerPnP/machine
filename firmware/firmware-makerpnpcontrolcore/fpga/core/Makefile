# MakerPnpControl-CORE
# yosys+nextpnr

YOSYS_ROOT := /usr/local/share/yosys
FLASH_ALGO_ROOT := ../../../../../dev-tools/makerpnpcore-w25q16jv

PROJECT  := core
TOP      := core_top
TB       := blink wake timer_mux clock_out quadspi
FAMILY   := ice40
TYPE     := hx8k
PACKAGE  := ct256
VERILOGS := core_top.v blink.v wake.v pll.v timer_mux.v clock_out.v memory.v quadspi.v encoders.v leds.v
COMMON_VLOGS := common.v memory.v encoders.v $(YOSYS_ROOT)/ice40/cells_sim.v


# helper function: build file list per TB
define TB_SRCS
$(1).v $(1)_tb.v $(COMMON_VLOGS)
endef

SIM_TARGETS := $(addprefix sim_,$(TB))

VVP_FILES := $(addsuffix .vvp,$(TB))
VCD_FILES := $(addsuffix .vcd,$(TB))

all: \
	$(PROJECT).bin \
	sim

# -------------------------
# SYNTHESIS (FPGA build)
# -------------------------

$(PROJECT).json: $(VERILOGS)
	yosys -q -l yosys.log -p 'synth_${FAMILY} -top $(TOP) -json $(PROJECT).json' $(VERILOGS)

$(PROJECT).asc: $(PROJECT).json pins.pcf
	nextpnr-${FAMILY} -q -l nextpnr.log --${TYPE} --package ${PACKAGE} --json $(PROJECT).json --pcf pins.pcf --asc $(PROJECT).asc
	@echo ""
	@grep -B 1 "%$$" nextpnr.log
	@echo ""

$(PROJECT).bin: $(PROJECT).asc
	icepack $(PROJECT).asc $(PROJECT).bin


# -------------------------
# SIMULATION (NEW)
# -------------------------

sim_%:
	iverilog -g2012 -o $*.vvp $(call TB_SRCS,$*)
	vvp $*.vvp

wave_%: sim_%
	gtkwave $*.vcd

# Optional: run all simulations
sim: $(SIM_TARGETS)

# -------------------------
# CLEAN
# -------------------------

clean:
	rm -rf $(PROJECT).bin $(PROJECT).asc $(PROJECT).json \
		yosys.log nextpnr.log abc.history \
		$(VVP_FILES) $(VCD_FILES)
# -------------------------
# LOAD FPGA
# -------------------------

load: $(PROJECT).bin
	 probe-rs download --chip makerpnpcore-w25q16jv --chip-description-path $(FLASH_ALGO_ROOT)/target/definition.yaml $(PROJECT).bin --binary-format=bin --base-address 0x70000000 --verify
