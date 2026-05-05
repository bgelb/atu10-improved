#ifndef ATU10_PROGRAMMER_BRIDGE_PROTOCOL_H
#define ATU10_PROGRAMMER_BRIDGE_PROTOCOL_H

#include <stdint.h>

#define PB_HID_PACKET_SIZE 64u
#define PB_MAX_PAYLOAD_SIZE 60u
#define PB_PROTOCOL_VERSION 2u
#define PB_MAGIC 0xa7u

typedef enum {
    PB_CMD_PROBE = 0x01u,
    PB_CMD_RESET_TARGET = 0x02u,
    PB_CMD_READ_TARGET_ID = 0x03u,
    PB_CMD_BEGIN_FLASH = 0x10u,
    PB_CMD_ERASE_ROW = 0x11u,
    PB_CMD_WRITE_CHUNK = 0x12u,
    PB_CMD_COMMIT_ROW = 0x13u,
    PB_CMD_VERIFY_RANGE = 0x14u,
    PB_CMD_RUN_TARGET = 0x15u,
    PB_CMD_READ_WORDS = 0x16u,
} pb_command_t;

typedef enum {
    PB_STATUS_OK = 0x00u,
    PB_STATUS_BUSY = 0x01u,
    PB_STATUS_BAD_REQUEST = 0x80u,
    PB_STATUS_VERIFY_FAILED = 0x81u,
    PB_STATUS_HARDWARE_FAULT = 0x82u,
} pb_status_t;

typedef enum {
    PB_MODE_IDLE = 0,
    PB_MODE_PROGRAMMING = 1,
    PB_MODE_RUNNING = 2,
} pb_mode_t;

typedef struct {
    pb_mode_t mode;
    uint32_t expected_rows;
    uint32_t committed_rows;
} pb_state_t;

typedef struct {
    void (*reset_target)(void *ctx);
    void (*enter_programming)(void *ctx);
    uint16_t (*read_target_id)(void *ctx);
    void (*erase_row)(void *ctx, uint32_t base_word_address);
    void (*write_chunk)(void *ctx, uint32_t base_word_address, uint8_t offset_bytes,
                        const uint8_t *data, uint8_t len);
    void (*commit_row)(void *ctx, uint32_t base_word_address);
    uint8_t (*verify_range)(void *ctx, uint32_t base_word_address, uint16_t word_count,
                            uint32_t expected_digest);
    uint8_t (*read_words)(void *ctx, uint32_t base_word_address, uint16_t word_count, uint8_t *out);
    void (*run_target)(void *ctx);
    void *ctx;
} pb_hal_t;

void pb_state_init(pb_state_t *state);
pb_status_t pb_handle_request(pb_state_t *state, const pb_hal_t *hal,
                              const uint8_t request[PB_HID_PACKET_SIZE],
                              uint8_t response[PB_HID_PACKET_SIZE]);

#endif
