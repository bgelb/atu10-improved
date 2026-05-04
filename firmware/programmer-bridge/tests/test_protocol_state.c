#include "bridge_protocol.h"
#include "unity.h"

#include <string.h>

typedef struct {
    unsigned resets;
    unsigned enters_programming;
    unsigned reads_id;
    unsigned erases;
    unsigned writes;
    unsigned commits;
    unsigned verifies;
    unsigned runs;
    uint32_t last_address;
    uint32_t last_digest;
    uint16_t last_word_count;
    uint8_t last_offset;
    uint8_t last_len;
} fake_hal_t;

static void reset_target(void *ctx) { ((fake_hal_t *)ctx)->resets++; }

static void enter_programming(void *ctx) { ((fake_hal_t *)ctx)->enters_programming++; }

static uint16_t read_target_id(void *ctx) {
    ((fake_hal_t *)ctx)->reads_id++;
    return 0x3075u;
}

static void erase_row(void *ctx, uint32_t base_word_address) {
    fake_hal_t *fake = (fake_hal_t *)ctx;
    fake->erases++;
    fake->last_address = base_word_address;
}

static void write_chunk(void *ctx, uint32_t base_word_address, uint8_t offset_bytes,
                        const uint8_t *data, uint8_t len) {
    fake_hal_t *fake = (fake_hal_t *)ctx;
    fake->writes++;
    fake->last_address = base_word_address;
    fake->last_offset = offset_bytes;
    fake->last_len = len;
    TEST_ASSERT_EQUAL_UINT8(0xaa, data[0]);
}

static void commit_row(void *ctx, uint32_t base_word_address) {
    fake_hal_t *fake = (fake_hal_t *)ctx;
    fake->commits++;
    fake->last_address = base_word_address;
}

static uint8_t verify_range(void *ctx, uint32_t base_word_address, uint16_t word_count,
                            uint32_t expected_digest) {
    fake_hal_t *fake = (fake_hal_t *)ctx;
    fake->verifies++;
    fake->last_address = base_word_address;
    fake->last_word_count = word_count;
    fake->last_digest = expected_digest;
    return 1u;
}

static void run_target(void *ctx) { ((fake_hal_t *)ctx)->runs++; }

static pb_hal_t make_hal(fake_hal_t *fake) {
    pb_hal_t hal;
    hal.reset_target = reset_target;
    hal.enter_programming = enter_programming;
    hal.read_target_id = read_target_id;
    hal.erase_row = erase_row;
    hal.write_chunk = write_chunk;
    hal.commit_row = commit_row;
    hal.verify_range = verify_range;
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

static void write_le32(uint8_t *data, uint32_t value) {
    data[0] = (uint8_t)(value & 0xffu);
    data[1] = (uint8_t)(value >> 8);
    data[2] = (uint8_t)(value >> 16);
    data[3] = (uint8_t)(value >> 24);
}

static void write_le16(uint8_t *data, uint16_t value) {
    data[0] = (uint8_t)(value & 0xffu);
    data[1] = (uint8_t)(value >> 8);
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
    TEST_ASSERT_EQUAL_UINT8(PB_PROTOCOL_VERSION, response[9]);
}

static void test_read_target_id_returns_little_endian_id(void) {
    pb_state_t state;
    fake_hal_t fake = {0};
    pb_hal_t hal = make_hal(&fake);
    uint8_t request[PB_HID_PACKET_SIZE];
    uint8_t response[PB_HID_PACKET_SIZE];

    pb_state_init(&state);
    fill_packet(request, 7u, PB_CMD_READ_TARGET_ID);

    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, &hal, request, response));
    TEST_ASSERT_EQUAL_INT(1, fake.reads_id);
    TEST_ASSERT_EQUAL_UINT8(2u, response[3]);
    TEST_ASSERT_EQUAL_UINT8(0x75u, response[4]);
    TEST_ASSERT_EQUAL_UINT8(0x30u, response[5]);
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

    fill_packet(request, 2u, PB_CMD_BEGIN_FLASH);
    request[3] = 4u;
    request[4] = 1u;
    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, &hal, request, response));
    TEST_ASSERT_EQUAL_INT(PB_MODE_PROGRAMMING, state.mode);
    TEST_ASSERT_EQUAL_INT(1, fake.enters_programming);

    fill_packet(request, 3u, PB_CMD_ERASE_ROW);
    request[3] = 4u;
    request[4] = 0x20u;
    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, &hal, request, response));
    TEST_ASSERT_EQUAL_INT(1, fake.erases);
    TEST_ASSERT_EQUAL_INT(0x20, fake.last_address);

    fill_packet(request, 4u, PB_CMD_WRITE_CHUNK);
    request[3] = 7u;
    request[4] = 0x20u;
    request[8] = 4u;
    request[9] = 0xaau;
    request[10] = 0xbbu;
    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, &hal, request, response));
    TEST_ASSERT_EQUAL_INT(1, fake.writes);
    TEST_ASSERT_EQUAL_INT(0x20, fake.last_address);
    TEST_ASSERT_EQUAL_INT(4, fake.last_offset);
    TEST_ASSERT_EQUAL_INT(2, fake.last_len);

    fill_packet(request, 5u, PB_CMD_COMMIT_ROW);
    request[3] = 4u;
    request[4] = 0x20u;
    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, &hal, request, response));
    TEST_ASSERT_EQUAL_INT(1, fake.commits);

    fill_packet(request, 6u, PB_CMD_VERIFY_RANGE);
    request[3] = 10u;
    write_le32(&request[4], 0x20u);
    write_le16(&request[8], 32u);
    write_le32(&request[10], 0x12345678u);
    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, &hal, request, response));
    TEST_ASSERT_EQUAL_INT(1, fake.verifies);
    TEST_ASSERT_EQUAL_INT(32, fake.last_word_count);
    TEST_ASSERT_EQUAL_INT(0x12345678u, fake.last_digest);

    fill_packet(request, 7u, PB_CMD_RUN_TARGET);
    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, &hal, request, response));
    TEST_ASSERT_EQUAL_INT(PB_MODE_RUNNING, state.mode);
    TEST_ASSERT_EQUAL_INT(1, fake.runs);
}

static void test_rejects_write_before_flash_session(void) {
    pb_state_t state;
    uint8_t request[PB_HID_PACKET_SIZE];
    uint8_t response[PB_HID_PACKET_SIZE];

    pb_state_init(&state);
    fill_packet(request, 1u, PB_CMD_WRITE_CHUNK);
    request[3] = 6u;

    TEST_ASSERT_EQUAL_INT(PB_STATUS_BAD_REQUEST,
                          pb_handle_request(&state, NULL, request, response));
}

static void test_verify_failure_surfaces_status(void) {
    pb_state_t state;
    uint8_t request[PB_HID_PACKET_SIZE];
    uint8_t response[PB_HID_PACKET_SIZE];

    pb_state_init(&state);
    fill_packet(request, 1u, PB_CMD_BEGIN_FLASH);
    request[3] = 4u;
    request[4] = 1u;
    TEST_ASSERT_EQUAL_INT(PB_STATUS_OK, pb_handle_request(&state, NULL, request, response));

    fill_packet(request, 2u, PB_CMD_VERIFY_RANGE);
    request[3] = 10u;
    TEST_ASSERT_EQUAL_INT(PB_STATUS_VERIFY_FAILED,
                          pb_handle_request(&state, NULL, request, response));
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_probe_returns_protocol_identity);
    RUN_TEST(test_read_target_id_returns_little_endian_id);
    RUN_TEST(test_flash_sequence_calls_hal_in_order_shape);
    RUN_TEST(test_rejects_write_before_flash_session);
    RUN_TEST(test_verify_failure_surfaces_status);
    return UNITY_END();
}
