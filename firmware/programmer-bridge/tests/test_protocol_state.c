#include "bridge_protocol.h"
#include "unity.h"

#include <string.h>

typedef struct {
    unsigned resets;
    unsigned enters_programming;
    unsigned programmed_rows;
    unsigned verifies;
    unsigned runs;
    uint32_t last_address;
    uint8_t last_len;
} fake_hal_t;

static void reset_target(void *ctx) { ((fake_hal_t *)ctx)->resets++; }

static void enter_programming(void *ctx) { ((fake_hal_t *)ctx)->enters_programming++; }

static void program_row(void *ctx, uint32_t address, const uint8_t *data, uint8_t len) {
    fake_hal_t *fake = (fake_hal_t *)ctx;
    fake->programmed_rows++;
    fake->last_address = address;
    fake->last_len = len;
    TEST_ASSERT_EQUAL_UINT8(0xaa, data[0]);
}

static uint8_t verify(void *ctx) {
    ((fake_hal_t *)ctx)->verifies++;
    return 1u;
}

static void run_target(void *ctx) { ((fake_hal_t *)ctx)->runs++; }

static pb_hal_t make_hal(fake_hal_t *fake) {
    pb_hal_t hal;
    hal.reset_target = reset_target;
    hal.enter_programming = enter_programming;
    hal.program_row = program_row;
    hal.verify = verify;
    hal.run_target = run_target;
    hal.ctx = fake;
    return hal;
}

static void fill_packet(uint8_t packet[PB_HID_PACKET_SIZE], uint8_t seq, uint8_t cmd) {
    memset(packet, 0, PB_HID_PACKET_SIZE);
    packet[0] = PB_MAGIC;
    packet[1] = seq;
    packet[2] = cmd;
}

static void test_probe_returns_protocol_identity(void) {
    pb_state_t state;
    uint8_t request[PB_HID_PACKET_SIZE];
    uint8_t response[PB_HID_PACKET_SIZE];

    pb_state_init(&state);
    fill_packet(request, 4u, PB_CMD_PROBE);

    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, NULL, request, response));
    TEST_ASSERT_EQUAL_UINT8(PB_MAGIC, response[0]);
    TEST_ASSERT_EQUAL_UINT8(4u, response[1]);
    TEST_ASSERT_EQUAL_UINT8(PB_STATUS_OK, response[2]);
    TEST_ASSERT_EQUAL_UINT8(6u, response[3]);
}

static void test_flash_sequence_calls_hal_in_order_shape(void) {
    pb_state_t state;
    fake_hal_t fake = {0};
    pb_hal_t hal = make_hal(&fake);
    uint8_t request[PB_HID_PACKET_SIZE];
    uint8_t response[PB_HID_PACKET_SIZE];

    pb_state_init(&state);

    fill_packet(request, 1u, PB_CMD_RESET_TARGET);
    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, &hal, request, response));
    TEST_ASSERT_EQUAL_INT(1, fake.resets);

    fill_packet(request, 2u, PB_CMD_START_FLASH);
    request[3] = 4u;
    request[4] = 1u;
    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, &hal, request, response));
    TEST_ASSERT_EQUAL_INT(PB_MODE_PROGRAMMING, state.mode);
    TEST_ASSERT_EQUAL_INT(1, fake.enters_programming);

    fill_packet(request, 3u, PB_CMD_PROGRAM_ROW);
    request[3] = 6u;
    request[4] = 0x20u;
    request[8] = 0xaau;
    request[9] = 0xbbu;
    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, &hal, request, response));
    TEST_ASSERT_EQUAL_INT(1, fake.programmed_rows);
    TEST_ASSERT_EQUAL_INT(0x20, fake.last_address);
    TEST_ASSERT_EQUAL_INT(2, fake.last_len);

    fill_packet(request, 4u, PB_CMD_VERIFY);
    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, &hal, request, response));
    TEST_ASSERT_EQUAL_INT(1, fake.verifies);

    fill_packet(request, 5u, PB_CMD_RUN_TARGET);
    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, &hal, request, response));
    TEST_ASSERT_EQUAL_INT(PB_MODE_RUNNING, state.mode);
    TEST_ASSERT_EQUAL_INT(1, fake.runs);
}

static void test_rejects_program_row_before_flash_session(void) {
    pb_state_t state;
    uint8_t request[PB_HID_PACKET_SIZE];
    uint8_t response[PB_HID_PACKET_SIZE];

    pb_state_init(&state);
    fill_packet(request, 1u, PB_CMD_PROGRAM_ROW);
    request[3] = 5u;

    TEST_ASSERT_EQUAL_INT(PB_STATUS_BAD_REQUEST,
                          pb_handle_request(&state, NULL, request, response));
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_probe_returns_protocol_identity);
    RUN_TEST(test_flash_sequence_calls_hal_in_order_shape);
    RUN_TEST(test_rejects_program_row_before_flash_session);
    return UNITY_END();
}
