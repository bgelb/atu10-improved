#include "board.h"
#include "uart_app.h"

#ifdef __XC8
#include <xc.h>
#endif

static void uart_write_byte(uint8_t byte) {
#if defined(__XC8) && TC_ENABLE_HW_UART
    while (PIR3bits.TXIF == 0u) {
    }
    TX1REG = byte;
#else
    (void)byte;
#endif
}

static uint8_t uart_read_byte(void) {
#if defined(__XC8) && TC_ENABLE_HW_UART
    while (PIR3bits.RCIF == 0u) {
    }
    if (RC1STAbits.OERR != 0u) {
        RC1STAbits.CREN = 0u;
        RC1STAbits.CREN = 1u;
    }
    return RC1REG;
#else
    return 0u;
#endif
}

static void uart_write_string(const char *text) {
    while (*text != '\0') {
        uart_write_byte((uint8_t)*text);
        text++;
    }
    uart_write_byte('\r');
    uart_write_byte('\n');
}

static void pps_unlock(void) {
#if defined(__XC8) && TC_ENABLE_HW_UART
    PPSLOCK = 0x55u;
    PPSLOCK = 0xaau;
    PPSLOCKbits.PPSLOCKED = 0u;
#endif
}

static void pps_lock(void) {
#if defined(__XC8) && TC_ENABLE_HW_UART
    PPSLOCK = 0x55u;
    PPSLOCK = 0xaau;
    PPSLOCKbits.PPSLOCKED = 1u;
#endif
}

static void board_init(void) {
#if defined(__XC8) && TC_ENABLE_HW_UART
    OSCCON1 = 0x60u;
    OSCFRQ = 0x06u;

    ANSELA = 0x00u;
    ANSELB = 0x00u;
    ANSELC = 0x00u;

    TRISBbits.TRISB6 = 0u;
    TRISBbits.TRISB7 = 1u;
    LATBbits.LATB6 = 1u;

    pps_unlock();
    RXPPS = TC_UART_RX_PPS_INPUT;
    TC_UART_TX_PPS_REGISTER = TC_UART_TX_PPS_FUNCTION;
    pps_lock();

    BAUD1CONbits.BRG16 = 1u;
    TX1STAbits.BRGH = 1u;
    SP1BRG = TC_UART_SPBRG_VALUE;
    RC1STAbits.SPEN = 1u;
    RC1STAbits.CREN = 1u;
    TX1STAbits.TXEN = 1u;
#endif
}

int main(void) {
    board_init();
    uart_write_string(tc_hello_line());

    for (;;) {
        uart_write_byte(tc_uart_process_byte(uart_read_byte()));
    }
}
