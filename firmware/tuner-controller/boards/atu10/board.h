#ifndef ATU10_TUNER_CONTROLLER_BOARD_H
#define ATU10_TUNER_CONTROLLER_BOARD_H

#define TC_BOARD_NAME "atu10"
#define TC_UART_BAUD 115200u

/*
 * Hardware confirmation needed:
 * The prompt source notes duplicate RB7 for two bridge connections. Confirm
 * the actual PIC16F18877 EUSART RX/TX pins before wiring PPS constants here.
 */
#define TC_TARGET_UART_RX_UNCONFIRMED 1
#define TC_TARGET_UART_TX_UNCONFIRMED 1

#endif
