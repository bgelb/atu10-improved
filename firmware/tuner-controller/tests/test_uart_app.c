#include "uart_app.h"
#include "unity.h"

static void test_hello_line_matches_serial_smoke_expectation(void) {
    TEST_ASSERT_EQUAL_STRING("ATU10-IMPROVED READY 115200", tc_hello_line());
}

static void test_uart_process_byte_echoes_input(void) {
    TEST_ASSERT_EQUAL_UINT8('p', tc_uart_process_byte('p'));
    TEST_ASSERT_EQUAL_UINT8('\n', tc_uart_process_byte('\n'));
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_hello_line_matches_serial_smoke_expectation);
    RUN_TEST(test_uart_process_byte_echoes_input);
    return UNITY_END();
}
