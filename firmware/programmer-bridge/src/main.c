#include "bridge_protocol.h"
#include "board.h"

#ifdef __XC8
#include <xc.h>
#endif

static pb_state_t g_state;

static void reset_target(void *ctx) {
    (void)ctx;
    /* TODO hardware bring-up: drive ATU10 target MCLR through the board circuit. */
}

static void enter_programming(void *ctx) {
    (void)ctx;
    /* TODO hardware bring-up: enter PIC16F18877 Program/Verify mode. */
}

static void program_row(void *ctx, uint32_t address, const uint8_t *data, uint8_t len) {
    (void)ctx;
    (void)address;
    (void)data;
    (void)len;
    /* TODO hardware bring-up: emit ICSP row programming sequence. */
}

static uint8_t verify(void *ctx) {
    (void)ctx;
    /* TODO hardware bring-up: compare target flash against host-provided image. */
    return 1u;
}

static void run_target(void *ctx) {
    (void)ctx;
    /* TODO hardware bring-up: release programming pins and reset target into run mode. */
}

int main(void) {
    const pb_hal_t hal = {
        reset_target, enter_programming, program_row, verify, run_target, 0,
    };

    pb_state_init(&g_state);
    (void)hal;

    for (;;) {
        /*
         * TODO USB bring-up: service composite CDC ACM + HID. HID packets call
         * pb_handle_request(); CDC bytes pass through to the target EUSART.
         */
    }
}
