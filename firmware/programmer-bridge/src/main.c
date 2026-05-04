#include "bridge_protocol.h"
#include "board.h"

#include <stdint.h>

#ifdef __XC8
#include <xc.h>
#endif

#define ICSP_CMD_LOAD_PC 0x80u
#define ICSP_CMD_ROW_ERASE 0xf0u
#define ICSP_CMD_LOAD_DATA 0x00u
#define ICSP_CMD_LOAD_DATA_INC 0x02u
#define ICSP_CMD_READ_DATA 0xfcu
#define ICSP_CMD_READ_DATA_INC 0xfeu
#define ICSP_CMD_BEGIN_INTERNALLY_TIMED 0xe0u

static pb_state_t g_state;
static uint8_t g_row_buffer[PB_TARGET_ROW_BYTES];

static void delay_cycles(unsigned count) {
    while (count-- > 0u) {
#ifdef __XC8
        __asm("nop");
#endif
    }
}

static void board_init(void) {
#if defined(__XC8)
    ANSELA = 0x00u;
    ANSELC = 0x00u;
    LATAbits.LATA4 = 1u;
    TRISAbits.TRISA4 = 0u;
    LATCbits.LATC4 = 0u;
    LATCbits.LATC5 = 0u;
    TRISCbits.TRISC4 = 1u;
    TRISCbits.TRISC5 = 1u;
#endif
}

static void icsp_data_output(uint8_t high) {
#if defined(__XC8)
    LATCbits.LATC4 = high != 0u ? 1u : 0u;
    TRISCbits.TRISC4 = 0u;
#else
    (void)high;
#endif
}

static void icsp_data_input(void) {
#if defined(__XC8)
    TRISCbits.TRISC4 = 1u;
#endif
}

static uint8_t icsp_data_read(void) {
#if defined(__XC8)
    return PORTCbits.RC4 != 0u ? 1u : 0u;
#else
    return 0u;
#endif
}

static void icsp_clock(uint8_t high) {
#if defined(__XC8)
    LATCbits.LATC5 = high != 0u ? 1u : 0u;
#else
    (void)high;
#endif
}

static void icsp_clock_output(void) {
#if defined(__XC8)
    TRISCbits.TRISC5 = 0u;
#endif
}

static void icsp_idle_lines(void) {
    icsp_data_output(0u);
    icsp_clock(0u);
    icsp_clock_output();
}

static void icsp_shift_out(uint32_t value, uint8_t bit_count) {
    while (bit_count > 0u) {
        bit_count--;
        icsp_data_output((uint8_t)((value >> bit_count) & 1u));
        delay_cycles(8u);
        icsp_clock(1u);
        delay_cycles(8u);
        icsp_clock(0u);
        delay_cycles(8u);
    }
}

static uint32_t icsp_shift_in(uint8_t bit_count) {
    uint32_t value = 0u;
    icsp_data_input();
    while (bit_count > 0u) {
        bit_count--;
        delay_cycles(8u);
        icsp_clock(1u);
        delay_cycles(8u);
        value = (value << 1) | icsp_data_read();
        icsp_clock(0u);
        delay_cycles(8u);
    }
    icsp_data_output(0u);
    return value;
}

static void icsp_command(uint8_t command) {
    icsp_shift_out(command, 8u);
    delay_cycles(20u);
}

static void icsp_payload_out(uint32_t payload) { icsp_shift_out(payload, 24u); }

static uint32_t icsp_payload_in(void) { return icsp_shift_in(24u); }

static void icsp_load_pc(uint32_t address) {
    icsp_command(ICSP_CMD_LOAD_PC);
    icsp_payload_out((address & 0xffffu) << 1);
}

static void icsp_load_word(uint16_t word, uint8_t increment) {
    icsp_command(increment != 0u ? ICSP_CMD_LOAD_DATA_INC : ICSP_CMD_LOAD_DATA);
    icsp_payload_out(((uint32_t)(word & 0x3fffu)) << 1);
}

static uint16_t icsp_read_word(uint8_t increment) {
    uint32_t payload;
    icsp_command(increment != 0u ? ICSP_CMD_READ_DATA_INC : ICSP_CMD_READ_DATA);
    payload = icsp_payload_in();
    return (uint16_t)((payload >> 1) & 0x3fffu);
}

static void icsp_begin_programming(void) {
    icsp_command(ICSP_CMD_BEGIN_INTERNALLY_TIMED);
    delay_cycles(12000u);
}

static void reset_target(void *ctx) {
    (void)ctx;
#if defined(__XC8) && PB_ENABLE_TARGET_RESET
    LATAbits.LATA4 = 0u;
    delay_cycles(PB_RESET_ASSERT_DELAY_CYCLES);
    LATAbits.LATA4 = 1u;
#endif
}

static void enter_programming(void *ctx) {
    (void)ctx;
#if defined(__XC8) && PB_ENABLE_TARGET_RESET
    LATAbits.LATA4 = 0u;
#endif
    icsp_idle_lines();
    delay_cycles(PB_RESET_ASSERT_DELAY_CYCLES);
    icsp_shift_out(0x4d434850u, 32u);
    delay_cycles(2000u);
}

static uint16_t read_target_id(void *ctx) {
    (void)ctx;
    enter_programming(ctx);
    icsp_load_pc(0x8006u);
    return icsp_read_word(0u);
}

static void erase_row(void *ctx, uint32_t base_word_address) {
    (void)ctx;
    icsp_load_pc(base_word_address);
    icsp_command(ICSP_CMD_ROW_ERASE);
    delay_cycles(12000u);
}

static void write_chunk(void *ctx, uint32_t base_word_address, uint8_t offset_bytes,
                        const uint8_t *data, uint8_t len) {
    uint8_t i;
    (void)ctx;
    (void)base_word_address;
    for (i = 0u; i < len && (uint16_t)offset_bytes + i < PB_TARGET_ROW_BYTES; i++) {
        g_row_buffer[(uint16_t)offset_bytes + i] = data[i];
    }
}

static void commit_row(void *ctx, uint32_t base_word_address) {
    uint8_t word_index;
    (void)ctx;
    icsp_load_pc(base_word_address);
    for (word_index = 0u; word_index < PB_TARGET_ROW_WORDS; word_index++) {
        uint16_t word = (uint16_t)g_row_buffer[(uint16_t)word_index * 2u] |
                        ((uint16_t)(g_row_buffer[((uint16_t)word_index * 2u) + 1u] & 0x3fu) << 8);
        icsp_load_word(word, word_index + 1u < PB_TARGET_ROW_WORDS ? 1u : 0u);
    }
    icsp_begin_programming();
}

static uint32_t digest_word(uint32_t hash, uint32_t address, uint16_t value) {
    uint32_t mixed = hash ^ address ^ (uint32_t)(value & 0x3fffu);
    return mixed * 0x01000193u;
}

static uint8_t verify_range(void *ctx, uint32_t base_word_address, uint16_t word_count,
                            uint32_t expected_digest) {
    uint16_t i;
    uint32_t digest = 0x811c9dc5u;
    (void)ctx;
    icsp_load_pc(base_word_address);
    for (i = 0u; i < word_count; i++) {
        digest = digest_word(digest, base_word_address + i,
                             icsp_read_word(i + 1u < word_count ? 1u : 0u));
    }
    return digest == expected_digest ? 1u : 0u;
}

static void run_target(void *ctx) {
    (void)ctx;
    icsp_data_input();
#if defined(__XC8)
    TRISCbits.TRISC5 = 1u;
#endif
    reset_target(ctx);
}

int main(void) {
    const pb_hal_t hal = {
        reset_target, enter_programming, read_target_id, erase_row, write_chunk,
        commit_row,   verify_range,      run_target,     0,
    };

    board_init();
    pb_state_init(&g_state);
    (void)hal;

    for (;;) {
        /*
         * USB runtime integration point:
         * - HID EP1 OUT packets call pb_handle_request(&g_state, &hal, ...)
         * - HID EP1 IN returns the response packet
         * - CDC EP2 bridges host bytes to/from the target UART on RC4/RC5 while
         *   PB_MODE_RUNNING.
         *
         * This repository currently contains descriptors and protocol logic, not
         * a complete PIC16F1454 USB device stack.
         */
    }
}
