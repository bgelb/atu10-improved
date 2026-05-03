#ifndef ATU10_TUNER_CONTROLLER_BOARD_H
#define ATU10_TUNER_CONTROLLER_BOARD_H

#define TC_BOARD_NAME "atu10"
#define TC_UART_BAUD 115200u
#define TC_FOSC_HZ 32000000UL
#define TC_UART_SPBRG_VALUE 68u
#define TC_ENABLE_HW_UART 0

/*
 * Hardware confirmation needed:
 * The prompt source notes duplicate RB7 for two bridge connections. Confirm
 * the actual PIC16F18877 EUSART RX/TX pins before wiring PPS constants here.
 *
 * Once confirmed, set TC_ENABLE_HW_UART to 1 and define:
 *   TC_UART_RX_PPS_INPUT: 5-bit PPS input code for the EUSART RX pin.
 *   TC_UART_TX_PPS_REGISTER: output PPS register for the EUSART TX pin.
 *   TC_UART_TX_PPS_FUNCTION: PPS output function code for EUSART TX.
 */
#define TC_TARGET_UART_RX_UNCONFIRMED 1
#define TC_TARGET_UART_TX_UNCONFIRMED 1

#endif
