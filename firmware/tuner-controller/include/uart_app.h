#ifndef ATU10_TUNER_CONTROLLER_UART_APP_H
#define ATU10_TUNER_CONTROLLER_UART_APP_H

#include <stdint.h>

#define TC_HELLO_LINE "ATU10-IMPROVED READY 115200"

const char *tc_hello_line(void);
uint8_t tc_uart_process_byte(uint8_t byte);

#endif
