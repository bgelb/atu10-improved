#include "board.h"
#include "uart_app.h"

#ifdef __XC8
#include <xc.h>
#endif

static void uart_write_byte(uint8_t byte) {
    (void)byte;
    /* TODO hardware bring-up: wait for EUSART TX ready and write the byte. */
}

static uint8_t uart_read_byte(void) {
    /* TODO hardware bring-up: wait for EUSART RX and return the received byte. */
    return 0u;
}

static void uart_write_string(const char *text) {
    while (*text != '\0') {
        uart_write_byte((uint8_t)*text);
        text++;
    }
    uart_write_byte('\r');
    uart_write_byte('\n');
}

static void board_init(void) {
    /*
     * TODO hardware bring-up: configure oscillator, PPS, and EUSART for
     * TC_UART_BAUD once the ATU10 target-side pins are confirmed.
     */
}

int main(void) {
    board_init();
    uart_write_string(tc_hello_line());

    for (;;) {
        uart_write_byte(tc_uart_process_byte(uart_read_byte()));
    }
}
