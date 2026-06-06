# MakerPnpControl-CORE
# yosys+nextpnr

YOSYS_ROOT := /usr/local/share/yosys
FLASH_ALGO_ROOT := ../../../../../dev-tools/makerpnpcore-w25q16jv

PROJECT  := core
TOP      := core_top
TB       := blink wake timer_mux clock_out quadspi leds
INT_TB   := int_leds_mem
FAMILY   := ice40
TYPE     := hx8k
PACKAGE  := ct256
VERILOGS := \
	core_top.v \
	la.v \
	blink.v \
	leds.v \
	buzzer.v \
	wake.v \
	pll.v \
	timer_mux.v \
	clock_out.v \
	memory.v \
	quadspi.v \
	encoders.v \

COMMON_SRCS := common.v $(YOSYS_ROOT)/ice40/cells_sim.v

# ----------------------------
# TESTBENCH SOURCE DEFINITIONS
# ----------------------------

# Regular TBs
EXTRA_SRCS_blink :=
EXTRA_SRCS_wake :=
EXTRA_SRCS_timer_mux :=
EXTRA_SRCS_clock_out :=
EXTRA_SRCS_quadspi := memory.v
EXTRA_SRCS_leds :=

# Special / integration TBs
EXTRA_SRCS_int_leds_mem := leds.v memory.v

define TB_SRCS
$1.v $1_tb.v $(COMMON_SRCS) $(EXTRA_SRCS_$1)
endef

define TB_SRCS_INT
$1_tb.v $(COMMON_SRCS) $(EXTRA_SRCS_$1)
endef

SIM_TARGETS := $(addprefix sim_,$(TB)) $(addprefix sim_int_,$(INT_TB))

VVP_FILES := $(addsuffix .vvp,$(TB)) $(addsuffix .vvp,$(INT_TB))
VCD_FILES := $(addsuffix .vcd,$(TB)) $(addsuffix .vvp,$(INT_TB))

all: \
	$(PROJECT).bin \
	sim \

# -------------------------
# SYNTHESIS (FPGA build)
# -------------------------

$(PROJECT).json: $(VERILOGS) Makefile
	yosys -q -l yosys.log -p 'synth_${FAMILY} -top $(TOP) -json $(PROJECT).json' $(VERILOGS)

$(PROJECT).asc: $(PROJECT).json pins.pcf
	nextpnr-${FAMILY} -q -l nextpnr.log --${TYPE} --package ${PACKAGE} --json $(PROJECT).json --pcf pins.pcf --asc $(PROJECT).asc
	@echo ""
	@grep -B 1 "%$$" nextpnr.log
	@echo ""

$(PROJECT).bin: $(PROJECT).asc
	icepack $(PROJECT).asc $(PROJECT).bin


# -------------------------
# SIMULATION
# -------------------------

sim_%:
	iverilog -g2012 -o $*.vvp $(call TB_SRCS,$*)
	vvp $*.vvp
	@echo ""

sim_int_%:
	iverilog -g2012 -o $*.vvp $(call TB_SRCS_INT,$*)
	vvp $*.vvp
	@echo ""

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
