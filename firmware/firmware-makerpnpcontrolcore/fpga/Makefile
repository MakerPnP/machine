# MakerPnpControl-CORE
# yosys+nextpnr

YOSYS_ROOT := /usr/local/share/yosys
FLASH_ALGO_ROOT := ../../../../dev-tools/makerpnpcore-w25q16jv

PROJECT  := core
TOP      := core_top
FAMILY   := ice40
TYPE     := hx8k
PACKAGE  := ct256

BUILD := build
SRC_MAIN := src/main
SRC_TEST := src/test

VERILOGS := $(shell find $(SRC_MAIN) -type f -name "*.v" ! -name "*_tb.v")

# Gather testbench paths
TB_FILES := $(shell find $(SRC_TEST) -type f -name "*_tb.v")

# Preserve relative paths: Strip the leading 'src/' prefix, then drop the extension/suffix
TB := $(patsubst src/%,%,$(patsubst %_tb.v,%,$(TB_FILES)))

COMMON_SRCS := $(SRC_MAIN)/common.v $(YOSYS_ROOT)/ice40/cells_sim.v

# Extract the unique generated subdirectory relative paths under build/
# e.g., "test/bar1/ test/bar2/"
GEN_DIRS := $(sort $(dir $(TB)))

# ----------------------------
# TESTBENCH SOURCE DEFINITIONS
# ----------------------------

# FUTURE consider using the testbench path somehow here, currently this prevents multi testbench files with
#        the same name in difference directories
# Regular TBs (Use the flat module name for variable lookups)
EXTRA_SRCS_blink :=
EXTRA_SRCS_wake :=
EXTRA_SRCS_timer_mux :=
EXTRA_SRCS_clock_out :=
EXTRA_SRCS_quadspi := $(SRC_MAIN)/registers/memory.v
EXTRA_SRCS_encoders := $(SRC_MAIN)/io/encoder.v
EXTRA_SRCS_leds :=

# Special / integration TBs
EXTRA_SRCS_int_leds_mem := $(SRC_MAIN)/io/leds.v $(SRC_MAIN)/registers/memory.v
EXTRA_SRCS_int_core_top := $(VERILOGS)

# Find RTL file matching module name
define FIND_RTL
$(shell find $(SRC_MAIN) -name "$1.v")
endef

# Reconstruct sources using the relative path ($1 is like 'test/bar1/foo')
define TB_SRCS
$(call FIND_RTL,$(notdir $1)) \
src/$1_tb.v \
$(COMMON_SRCS) \
$(EXTRA_SRCS_$(notdir $1))
endef

# Simulation targets match the relative paths: sim/test/bar1/foo and sim_int/test/bar1/int_foo
SIM_TARGETS := $(addprefix sim/,$(TB))

## all: Default target. Synthesizes the FPGA binary and builds simulations.
all: \
	$(BUILD)/$(PROJECT).bin \
	sim

# -------------------------
# SYNTHESIS (FPGA build)
# -------------------------
$(BUILD):
	mkdir -p $(BUILD)

$(BUILD)/$(PROJECT)-design.json: $(VERILOGS) $(BUILD) Makefile
	yosys -q -l $(BUILD)/yosys.log -p 'synth_${FAMILY} -top $(TOP) -json $(BUILD)/$(PROJECT)-design.json' $(VERILOGS)

$(BUILD)/$(PROJECT).asc: $(BUILD)/$(PROJECT)-design.json pins.pcf
	nextpnr-${FAMILY} -q -l $(BUILD)/nextpnr.log --${TYPE} --package ${PACKAGE} --top $(TOP) --json $(BUILD)/$(PROJECT)-design.json --write $(BUILD)/$(PROJECT)-place.json --report $(BUILD)/$(PROJECT)-report.json --pcf pins.pcf --asc $(BUILD)/$(PROJECT).asc --placed-svg $(BUILD)/$(PROJECT)-placed.svg --freq 100 -v
	@echo ""
	@grep -B 1 "%$$" $(BUILD)/nextpnr.log
	@echo ""

$(BUILD)/$(PROJECT).bin: $(BUILD)/$(PROJECT).asc
	icepack $(BUILD)/$(PROJECT).asc $(BUILD)/$(PROJECT).bin

## synthesise the bitstream (binary)
synth: $(BUILD)/$(PROJECT).bin

# -------------------------
# SIMULATION
# -------------------------

## sim/<path_to_testbench>: Run a specific testbench preserving relative path layout, e.g. sim/test/bar1/foo
sim/%:
	@mkdir -p $(BUILD)/$(dir $*)
	iverilog -D SIM -g2012 -o $(BUILD)/$*_tb.vvp $(call TB_SRCS,$*)
	cd $(BUILD)/$(dir $*) && vvp -n $(notdir $*_tb.vvp)
	@echo ""

## wave_gtk_<path_to_testbench>: Run simulation and open gtkwave for a regular target.
wave_gtk/%: sim/%
	gtkwave $(BUILD)/$*_tb.vcd

## wave_<path_to_testbench>: Run simulation and open surfer for a regular target.
wave/%: sim/%
	surfer $(BUILD)/$*_tb.vcd

## sim: Execute all discovered regular and integration simulation targets.
sim: $(SIM_TARGETS)

# -------------------------
# CLEAN
# -------------------------

## clean: Surgically removes only generated compilation and simulation assets without deleting all of build/.
clean:
	# 1. Remove specific synthesis top-level files
	rm -f $(BUILD)/$(PROJECT).bin $(BUILD)/$(PROJECT).asc $(BUILD)/$(PROJECT).json \
		$(BUILD)/yosys.log $(BUILD)/nextpnr.log
	# 2. Safely remove only the test directories generated under build/
	@for dir in $(GEN_DIRS); do \
		if [ -d "$(BUILD)/$$dir" ]; then \
			echo "Removing $(BUILD)/$$dir"; \
			rm -rf "$(BUILD)/$$dir"; \
		fi; \
	done
	# 3. Clean up empty parent directories up to $(BUILD) without touching $(BUILD) itself
	@find $(BUILD) -type d -empty -delete 2>/dev/null || true

# -------------------------
# LOAD FPGA
# -------------------------

## load: Flash the synthesized core binary onto the hardware chip target using probe-rs.
load: $(BUILD)/$(PROJECT).bin
	probe-rs download --chip makerpnpcore-w25q16jv --chip-description-path $(FLASH_ALGO_ROOT)/target/definition.yaml $(BUILD)/$(PROJECT).bin --binary-format=bin --base-address 0x70000000 --verify

# -------------------------
# HELP TARGET
# -------------------------

## help: Display this help message containing descriptions of available targets and valid simulation paths.
help:
	@printf "Available Makefile targets:\n\n"
	@awk '/^[a-zA-Z\-_0-9%\/]+:/ { \
		helpMessage = match(lastLine, /^## (.*)/); \
		if (helpMessage) { \
			helpOut = substr(lastLine, RSTART + 3, RLENGTH); \
			gsub(/:.*/, "", $$1); \
			printf "  \033[36m%-35s\033[0m %s\n", $$1, helpOut; \
			} \
		} \
		{ lastLine = $$0 }' $(MAKEFILE_LIST)
	@printf "\n\033[1;34mValid testbench paths for sim/%%, wave/%%, and wave_gtk/%%:\033[0m\n"
	@for path in $(TB); do \
		printf "  %s\n" "$$path"; \
	done
	@printf "\n"

.PHONY: all sim clean load help
