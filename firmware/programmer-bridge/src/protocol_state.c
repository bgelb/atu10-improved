#include "bridge_protocol.h"

#include <stddef.h>
#include <string.h>

static void write_response(uint8_t sequence, pb_status_t status, const uint8_t *payload,
                           uint8_t payload_len, uint8_t response[PB_HID_PACKET_SIZE]) {
    memset(response, 0, PB_HID_PACKET_SIZE);
    response[0] = PB_MAGIC;
    response[1] = sequence;
    response[2] = (uint8_t)status;
    response[3] = payload_len;
    if (payload_len > 0u && payload != NULL) {
        memcpy(&response[4], payload, payload_len);
    }
}

static uint32_t read_le32(const uint8_t *data) {
    return ((uint32_t)data[0]) | ((uint32_t)data[1] << 8) | ((uint32_t)data[2] << 16) |
           ((uint32_t)data[3] << 24);
}

static uint16_t read_le16(const uint8_t *data) {
    return (uint16_t)(((uint16_t)data[0]) | ((uint16_t)data[1] << 8));
}

void pb_state_init(pb_state_t *state) {
    state->mode = PB_MODE_IDLE;
    state->expected_rows = 0u;
    state->committed_rows = 0u;
}

pb_status_t pb_handle_request(pb_state_t *state, const pb_hal_t *hal,
                              const uint8_t request[PB_HID_PACKET_SIZE],
                              uint8_t response[PB_HID_PACKET_SIZE]) {
    uint8_t sequence = request[1];
    uint8_t command = request[2];
    uint8_t payload_len = request[3];
    const uint8_t *payload = &request[4];
    pb_status_t status = PB_STATUS_OK;

    if (request[0] != PB_MAGIC || payload_len > PB_MAX_PAYLOAD_SIZE) {
        write_response(sequence, PB_STATUS_BAD_REQUEST, NULL, 0u, response);
        return PB_STATUS_BAD_REQUEST;
    }

    switch (command) {
    case PB_CMD_PROBE: {
        const uint8_t probe_payload[] = {'A', 'T', 'U', '1', '0', PB_PROTOCOL_VERSION};
        write_response(sequence, PB_STATUS_OK, probe_payload, sizeof(probe_payload), response);
        return PB_STATUS_OK;
    }

    case PB_CMD_RESET_TARGET:
        if (payload_len != 0u) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        if (hal != NULL && hal->reset_target != NULL) {
            hal->reset_target(hal->ctx);
        }
        state->mode = PB_MODE_IDLE;
        break;

    case PB_CMD_READ_TARGET_ID: {
        uint16_t target_id = 0u;
        uint8_t target_payload[2];
        if (payload_len != 0u) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        if (hal == NULL || hal->read_target_id == NULL) {
            status = PB_STATUS_HARDWARE_FAULT;
            break;
        }
        target_id = hal->read_target_id(hal->ctx);
        state->mode = PB_MODE_PROGRAMMING;
        target_payload[0] = (uint8_t)(target_id & 0xffu);
        target_payload[1] = (uint8_t)(target_id >> 8);
        write_response(sequence, PB_STATUS_OK, target_payload, sizeof(target_payload), response);
        return PB_STATUS_OK;
    }

    case PB_CMD_BEGIN_FLASH:
        if (payload_len != 4u) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        state->expected_rows = read_le32(payload);
        state->committed_rows = 0u;
        if (state->mode != PB_MODE_PROGRAMMING && hal != NULL && hal->enter_programming != NULL) {
            hal->enter_programming(hal->ctx);
        }
        state->mode = PB_MODE_PROGRAMMING;
        break;

    case PB_CMD_ERASE_ROW:
        if (state->mode != PB_MODE_PROGRAMMING || payload_len != 4u) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        if (hal != NULL && hal->erase_row != NULL) {
            hal->erase_row(hal->ctx, read_le32(payload));
        }
        break;

    case PB_CMD_WRITE_CHUNK:
        if (state->mode != PB_MODE_PROGRAMMING || payload_len < 6u) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        if (hal != NULL && hal->write_chunk != NULL) {
            hal->write_chunk(hal->ctx, read_le32(payload), payload[4], &payload[5],
                             (uint8_t)(payload_len - 5u));
        }
        break;

    case PB_CMD_COMMIT_ROW:
        if (state->mode != PB_MODE_PROGRAMMING || payload_len != 4u) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        if (hal != NULL && hal->commit_row != NULL) {
            hal->commit_row(hal->ctx, read_le32(payload));
        }
        state->committed_rows++;
        break;

    case PB_CMD_VERIFY_RANGE:
        if (state->mode != PB_MODE_PROGRAMMING || payload_len != 10u) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        if (state->committed_rows > state->expected_rows) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        if (hal == NULL || hal->verify_range == NULL ||
            hal->verify_range(hal->ctx, read_le32(payload), read_le16(&payload[4]),
                              read_le32(&payload[6])) == 0u) {
            status = PB_STATUS_VERIFY_FAILED;
            break;
        }
        break;

    case PB_CMD_RUN_TARGET:
        if (payload_len != 0u) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        if (hal != NULL && hal->run_target != NULL) {
            hal->run_target(hal->ctx);
        }
        state->mode = PB_MODE_RUNNING;
        break;

    case PB_CMD_READ_WORDS: {
        uint16_t word_count;
        uint8_t read_payload[PB_MAX_PAYLOAD_SIZE];
        if (payload_len != 6u) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        word_count = read_le16(&payload[4]);
        if (word_count == 0u || word_count > (PB_MAX_PAYLOAD_SIZE / 2u) || hal == NULL ||
            hal->read_words == NULL) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        if (state->mode != PB_MODE_PROGRAMMING) {
            if (hal->enter_programming != NULL) {
                hal->enter_programming(hal->ctx);
            }
            state->mode = PB_MODE_PROGRAMMING;
        }
        if (hal->read_words(hal->ctx, read_le32(payload), word_count, read_payload) == 0u) {
            status = PB_STATUS_HARDWARE_FAULT;
            break;
        }
        write_response(sequence, PB_STATUS_OK, read_payload, (uint8_t)(word_count * 2u), response);
        return PB_STATUS_OK;
    }

    default:
        status = PB_STATUS_BAD_REQUEST;
        break;
    }

    write_response(sequence, status, NULL, 0u, response);
    return status;
}
