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

void pb_state_init(pb_state_t *state) {
    state->mode = PB_MODE_IDLE;
    state->expected_rows = 0u;
    state->programmed_rows = 0u;
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

    case PB_CMD_START_FLASH:
        if (payload_len != 4u) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        state->expected_rows = read_le32(payload);
        state->programmed_rows = 0u;
        state->mode = PB_MODE_PROGRAMMING;
        if (hal != NULL && hal->enter_programming != NULL) {
            hal->enter_programming(hal->ctx);
        }
        break;

    case PB_CMD_PROGRAM_ROW:
        if (state->mode != PB_MODE_PROGRAMMING || payload_len < 5u) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        if (hal != NULL && hal->program_row != NULL) {
            hal->program_row(hal->ctx, read_le32(payload), &payload[4],
                             (uint8_t)(payload_len - 4u));
        }
        state->programmed_rows++;
        break;

    case PB_CMD_VERIFY:
        if (state->mode != PB_MODE_PROGRAMMING) {
            status = PB_STATUS_BAD_REQUEST;
            break;
        }
        if (hal != NULL && hal->verify != NULL && hal->verify(hal->ctx) == 0u) {
            status = PB_STATUS_VERIFY_FAILED;
            break;
        }
        break;

    case PB_CMD_RUN_TARGET:
        if (hal != NULL && hal->run_target != NULL) {
            hal->run_target(hal->ctx);
        }
        state->mode = PB_MODE_RUNNING;
        break;

    default:
        status = PB_STATUS_BAD_REQUEST;
        break;
    }

    write_response(sequence, status, NULL, 0u, response);
    return status;
}
