#ifndef ATU10_PROGRAMMER_BRIDGE_BOARD_H
#define ATU10_PROGRAMMER_BRIDGE_BOARD_H

#define PB_BOARD_NAME "atu10"
#define PB_TARGET_UART_BAUD 115200u
#define PB_ENABLE_TARGET_RESET 1
#define PB_RESET_ASSERT_DELAY_CYCLES 2000u

/*
 * Hardware confirmation needed:
 * Source notes mention RC4<->RB7 and RC5<->RB7, which repeats RB7 on the
 * tuner-controller side. Keep all uncertain nets centralized here until the
 * board is traced or the schematic is confirmed.
 */
#define PB_PIN_TARGET_MCLR_RA4 1
#define PB_PIN_TARGET_MCLR_RA4_ACTIVE_HIGH 1
#define PB_PIN_TARGET_ICSPDAT_RC4 1
#define PB_PIN_TARGET_ICSPCLK_RC5 1
#define PB_PIN_TARGET_UART_TX_RC4 1
#define PB_PIN_TARGET_UART_RX_RC5 1

#define PB_TARGET_EXPECTED_DEVICE_ID 0x3075u
#define PB_TARGET_ROW_WORDS 32u
#define PB_TARGET_ROW_BYTES (PB_TARGET_ROW_WORDS * 2u)

#endif
