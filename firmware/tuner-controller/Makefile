XC8 ?= xc8-cc
MCU ?= 16F18877
PIC18877_DFP ?= $(HOME)/.mchp_packs/Microchip/PIC16F1xxxx_DFP/1.31.465/xc8
BUILD_DIR := build

CFLAGS := -mcpu=$(MCU) -mdfp=$(PIC18877_DFP) -std=c99 -Wall -Wextra
INCLUDES := -Iinclude -Iboards/atu10
SOURCES := src/main.c src/uart_app.c

.PHONY: build clean

build:
	mkdir -p $(BUILD_DIR)
	$(XC8) $(CFLAGS) $(INCLUDES) $(SOURCES) -o $(BUILD_DIR)/tuner-controller.hex

clean:
	rm -rf $(BUILD_DIR)
