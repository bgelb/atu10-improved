XC8 ?= xc8-cc
MCU ?= 16F1454
PIC1454_DFP ?= $(HOME)/.mchp_packs/Microchip/PIC12-16F1xxx_DFP/1.9.258/xc8
BUILD_DIR := build

CFLAGS := -mcpu=$(MCU) -mdfp=$(PIC1454_DFP) -std=c99 -Wall -Wextra
INCLUDES := -Iinclude -Iboards/atu10
SOURCES := src/main.c src/protocol_state.c

.PHONY: build clean

build:
	mkdir -p $(BUILD_DIR)
	$(XC8) $(CFLAGS) $(INCLUDES) $(SOURCES) -o $(BUILD_DIR)/programmer-bridge.hex

clean:
	rm -rf $(BUILD_DIR)
