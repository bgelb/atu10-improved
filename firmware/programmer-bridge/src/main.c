#include "bridge_protocol.h"
#include "board.h"
#include "usb_descriptors.h"

#include <stdint.h>
#include <string.h>

#ifdef __XC8
#include <xc.h>
#endif

#define ICSP_CMD_LOAD_PC 0x80u
#define ICSP_CMD_ROW_ERASE 0xf0u
#define ICSP_CMD_LOAD_DATA 0x00u
#define ICSP_CMD_LOAD_DATA_INC 0x02u
#define ICSP_CMD_READ_DATA 0xfcu
#define ICSP_CMD_READ_DATA_INC 0xfeu
#define ICSP_CMD_BEGIN_INTERNALLY_TIMED 0xe0u

#define USB_REQ_GET_STATUS 0x00u
#define USB_REQ_CLEAR_FEATURE 0x01u
#define USB_REQ_SET_FEATURE 0x03u
#define USB_REQ_SET_ADDRESS 0x05u
#define USB_REQ_GET_DESCRIPTOR 0x06u
#define USB_REQ_GET_CONFIGURATION 0x08u
#define USB_REQ_SET_CONFIGURATION 0x09u
#define USB_REQ_GET_INTERFACE 0x0au
#define USB_REQ_SET_INTERFACE 0x0bu

#define USB_DESC_DEVICE 0x01u
#define USB_DESC_CONFIGURATION 0x02u
#define USB_DESC_STRING 0x03u
#define USB_DESC_HID 0x21u
#define USB_DESC_HID_REPORT 0x22u

#define CDC_REQ_SET_LINE_CODING 0x20u
#define CDC_REQ_GET_LINE_CODING 0x21u
#define CDC_REQ_SET_CONTROL_LINE_STATE 0x22u

#define HID_REQ_GET_REPORT 0x01u
#define HID_REQ_SET_IDLE 0x0au

#define USB_BD_UOWN 0x80u
#define USB_BD_DTS 0x40u
#define USB_BD_DTSEN 0x08u
#define USB_BD_BSTALL 0x04u

#define USB_EP0 0u
#define USB_EP_HID 1u
#define USB_EP_CDC 2u
#define USB_EP_CDC_NOTIFY 3u

#define USB_DIR_OUT 0u
#define USB_DIR_IN 1u

#define USB_BDT_ENTRY_COUNT 32u
#define USB_BDT_INDEX(ep, dir) (((ep) * 2u) + (dir))
#define USB_UEP_CONTROL_BIDIR 0x16u
#define USB_UEP_DATA_BIDIR 0x1eu
#define USB_UEP_DATA_IN 0x1au

#define USB_ADDR_EP0_OUT 0x2080u
#define USB_ADDR_EP0_IN 0x2088u
#define USB_ADDR_HID_OUT 0x2090u
#define USB_ADDR_HID_IN 0x20d0u
#define USB_ADDR_CDC_OUT 0x2110u
#define USB_ADDR_CDC_IN 0x2150u
#define USB_ADDR_CDC_NOTIFY_IN USB_ADDR_EP0_IN
#define USB_HID_DESCRIPTOR_OFFSET 84u
#define ICSP_ERASE_WRITE_DELAY_CYCLES 60000u
#define UART_RX_BUFFER_SIZE 64u

static pb_state_t g_state;
static uint8_t g_row_buffer[PB_TARGET_ROW_BYTES];
static uint8_t g_uart_rx_buffer[UART_RX_BUFFER_SIZE];
static uint8_t g_uart_rx_head;
static uint8_t g_uart_rx_tail;

typedef struct {
    volatile uint8_t stat;
    volatile uint8_t cnt;
    volatile uint8_t adrl;
    volatile uint8_t adrh;
} usb_bd_t;

#if defined(__XC8)
static volatile usb_bd_t g_bdt[USB_BDT_ENTRY_COUNT] __at(0x2000);
static volatile uint8_t g_ep0_out[PB_USB_EP0_SIZE] __at(USB_ADDR_EP0_OUT);
static volatile uint8_t g_ep0_in[PB_USB_EP0_SIZE] __at(USB_ADDR_EP0_IN);
static volatile uint8_t g_hid_out[PB_USB_HID_EP_SIZE] __at(USB_ADDR_HID_OUT);
static volatile uint8_t g_hid_in[PB_USB_HID_EP_SIZE] __at(USB_ADDR_HID_IN);
static volatile uint8_t g_cdc_out[PB_USB_CDC_EP_SIZE] __at(USB_ADDR_CDC_OUT);
static volatile uint8_t g_cdc_in[PB_USB_CDC_EP_SIZE] __at(USB_ADDR_CDC_IN);
#else
static volatile usb_bd_t g_bdt[USB_BDT_ENTRY_COUNT];
static volatile uint8_t g_ep0_out[PB_USB_EP0_SIZE];
static volatile uint8_t g_ep0_in[PB_USB_EP0_SIZE];
static volatile uint8_t g_hid_out[PB_USB_HID_EP_SIZE];
static volatile uint8_t g_hid_in[PB_USB_HID_EP_SIZE];
static volatile uint8_t g_cdc_out[PB_USB_CDC_EP_SIZE];
static volatile uint8_t g_cdc_in[PB_USB_CDC_EP_SIZE];
#endif

static const uint8_t g_string0[] = {4, USB_DESC_STRING, 0x09, 0x04};
static const uint8_t g_string_manufacturer[] = {
    20, USB_DESC_STRING, 'A', 0, 'T', 0, 'U', 0, '1', 0, '0', 0, ' ', 0, 'D', 0, 'e', 0, 'v', 0,
};
static const uint8_t g_string_product[] = {
    32,  USB_DESC_STRING,
    'A', 0,
    'T', 0,
    'U', 0,
    '1', 0,
    '0', 0,
    ' ', 0,
    'B', 0,
    'r', 0,
    'i', 0,
    'd', 0,
    'g', 0,
    'e', 0,
    ' ', 0,
    'v', 0,
    '2', 0,
};
static const uint8_t g_string_serial[] = {
    18, USB_DESC_STRING, 'A', 0, '7', 0, '1', 0, '0', 0, '0', 0, '0', 0, '0', 0, '1', 0,
};
static const uint8_t g_cdc_line_coding[] = {
    0x00, 0xc2, 0x01, 0x00, /* 115200 baud */
    0x00,                   /* one stop bit */
    0x00,                   /* no parity */
    0x08,                   /* 8 data bits */
};

static const uint8_t *g_ctrl_data;
static uint16_t g_ctrl_remaining;
static uint8_t g_pending_address;
static uint8_t g_address_pending;
static uint8_t g_configuration;
static uint8_t g_ep0_out_data_stage;
static uint8_t g_ep0_in_dts;
static uint8_t g_ep0_out_dts;
static uint8_t g_hid_in_dts;
static uint8_t g_hid_out_dts;
static uint8_t g_cdc_in_dts;
static uint8_t g_cdc_out_dts;
static uint8_t g_cdc_control_packet[PB_HID_PACKET_SIZE];
static uint8_t g_cdc_control_len;

static void delay_cycles(unsigned count) {
    while (count-- > 0u) {
#ifdef __XC8
        __asm("nop");
#endif
    }
}

static void board_init(void) {
#if defined(__XC8)
    OSCCON = 0xfcu;
    delay_cycles(24000u);
    ACTCON = 0x90u;
    ANSELA = 0x00u;
    ANSELC = 0x00u;
    LATAbits.LATA4 = 0u;
    TRISAbits.TRISA4 = 0u;
    LATCbits.LATC4 = 0u;
    LATCbits.LATC5 = 0u;
    TRISCbits.TRISC4 = 1u;
    TRISCbits.TRISC5 = 1u;
#endif
}

static void target_uart_disable(void) {
#if defined(__XC8)
    RCSTAbits.SPEN = 0u;
    TXSTAbits.TXEN = 0u;
#endif
}

static uint8_t uart_rx_next(uint8_t index) {
    index++;
    if (index >= UART_RX_BUFFER_SIZE) {
        index = 0u;
    }
    return index;
}

static void uart_rx_clear(void) {
    g_uart_rx_head = 0u;
    g_uart_rx_tail = 0u;
}

static void uart_rx_push(uint8_t byte) {
    uint8_t next = uart_rx_next(g_uart_rx_head);
    if (next != g_uart_rx_tail) {
        g_uart_rx_buffer[g_uart_rx_head] = byte;
        g_uart_rx_head = next;
    }
}

static uint8_t uart_rx_pop(uint8_t *byte) {
    if (g_uart_rx_tail == g_uart_rx_head) {
        return 0u;
    }
    *byte = g_uart_rx_buffer[g_uart_rx_tail];
    g_uart_rx_tail = uart_rx_next(g_uart_rx_tail);
    return 1u;
}

static void target_uart_init(void) {
    uart_rx_clear();
#if defined(__XC8)
    TRISCbits.TRISC4 = 0u;
    TRISCbits.TRISC5 = 1u;
    APFCON = 0x00u;
    BAUDCONbits.BRG16 = 1u;
    TXSTAbits.BRGH = 1u;
    SPBRGH = 0u;
    SPBRGL = 103u;
    TXSTAbits.SYNC = 0u;
    RCSTAbits.SPEN = 1u;
    TXSTAbits.TXEN = 1u;
    RCSTAbits.CREN = 1u;
#endif
}

static void icsp_data_output(uint8_t high) {
#if defined(__XC8)
    LATCbits.LATC4 = high != 0u ? 1u : 0u;
    TRISCbits.TRISC4 = 0u;
#else
    (void)high;
#endif
}

static void icsp_data_input(void) {
#if defined(__XC8)
    TRISCbits.TRISC4 = 1u;
#endif
}

static uint8_t icsp_data_read(void) {
#if defined(__XC8)
    return PORTCbits.RC4 != 0u ? 1u : 0u;
#else
    return 0u;
#endif
}

static void icsp_clock(uint8_t high) {
#if defined(__XC8)
    LATCbits.LATC5 = high != 0u ? 1u : 0u;
#else
    (void)high;
#endif
}

static void icsp_clock_output(void) {
#if defined(__XC8)
    TRISCbits.TRISC5 = 0u;
#endif
}

static void icsp_idle_lines(void) {
    icsp_data_output(0u);
    icsp_clock(0u);
    icsp_clock_output();
}

static void icsp_shift_out(uint32_t value, uint8_t bit_count) {
    while (bit_count > 0u) {
        bit_count--;
        icsp_data_output((uint8_t)((value >> bit_count) & 1u));
        delay_cycles(8u);
        icsp_clock(1u);
        delay_cycles(8u);
        icsp_clock(0u);
        delay_cycles(8u);
    }
}

static uint32_t icsp_shift_in(uint8_t bit_count) {
    uint32_t value = 0u;
    icsp_data_input();
    while (bit_count > 0u) {
        bit_count--;
        delay_cycles(8u);
        icsp_clock(1u);
        delay_cycles(8u);
        icsp_clock(0u);
        delay_cycles(8u);
        value = (value << 1) | icsp_data_read();
    }
    icsp_data_output(0u);
    return value;
}

static void icsp_command(uint8_t command) {
    icsp_shift_out(command, 8u);
    delay_cycles(20u);
}

static void icsp_payload_out(uint32_t payload) { icsp_shift_out(payload, 24u); }

static uint32_t icsp_payload_in(void) { return icsp_shift_in(24u); }

static void icsp_load_pc(uint32_t address) {
    icsp_command(ICSP_CMD_LOAD_PC);
    icsp_payload_out((address & 0xffffu) << 1);
}

static void icsp_load_word(uint16_t word, uint8_t increment) {
    icsp_command(increment != 0u ? ICSP_CMD_LOAD_DATA_INC : ICSP_CMD_LOAD_DATA);
    icsp_payload_out(((uint32_t)(word & 0x3fffu)) << 1);
}

static uint16_t icsp_read_word(uint8_t increment) {
    uint32_t payload;
    icsp_command(increment != 0u ? ICSP_CMD_READ_DATA_INC : ICSP_CMD_READ_DATA);
    payload = icsp_payload_in();
    return (uint16_t)((payload >> 1) & 0x3fffu);
}

static void icsp_begin_programming(void) {
    icsp_command(ICSP_CMD_BEGIN_INTERNALLY_TIMED);
    delay_cycles(ICSP_ERASE_WRITE_DELAY_CYCLES);
}

static void reset_target(void *ctx) {
    (void)ctx;
#if defined(__XC8) && PB_ENABLE_TARGET_RESET
    LATAbits.LATA4 = 1u;
    delay_cycles(PB_RESET_ASSERT_DELAY_CYCLES);
    LATAbits.LATA4 = 0u;
#endif
}

static void enter_programming(void *ctx) {
    (void)ctx;
    target_uart_disable();
    icsp_idle_lines();
#if defined(__XC8) && PB_ENABLE_TARGET_RESET
    LATAbits.LATA4 = 1u;
#endif
    delay_cycles(PB_RESET_ASSERT_DELAY_CYCLES);
    icsp_shift_out(0x4d434850u, 32u);
    delay_cycles(2000u);
}

static uint16_t read_target_id(void *ctx) {
    (void)ctx;
    enter_programming(ctx);
    icsp_load_pc(0x8006u);
    return icsp_read_word(0u);
}

static void erase_row(void *ctx, uint32_t base_word_address) {
    (void)ctx;
    icsp_load_pc(base_word_address);
    icsp_command(ICSP_CMD_ROW_ERASE);
    delay_cycles(ICSP_ERASE_WRITE_DELAY_CYCLES);
}

static void write_chunk(void *ctx, uint32_t base_word_address, uint8_t offset_bytes,
                        const uint8_t *data, uint8_t len) {
    uint8_t i;
    (void)ctx;
    (void)base_word_address;
    for (i = 0u; i < len && (uint16_t)offset_bytes + i < PB_TARGET_ROW_BYTES; i++) {
        g_row_buffer[(uint16_t)offset_bytes + i] = data[i];
    }
}

static void commit_row(void *ctx, uint32_t base_word_address) {
    uint8_t word_index;
    (void)ctx;
    icsp_load_pc(base_word_address);
    for (word_index = 0u; word_index < PB_TARGET_ROW_WORDS; word_index++) {
        uint16_t word = (uint16_t)g_row_buffer[(uint16_t)word_index * 2u] |
                        ((uint16_t)(g_row_buffer[((uint16_t)word_index * 2u) + 1u] & 0x3fu) << 8);
        icsp_load_word(word, word_index + 1u < PB_TARGET_ROW_WORDS ? 1u : 0u);
    }
    icsp_begin_programming();
}

static uint32_t digest_word(uint32_t hash, uint32_t address, uint16_t value) {
    uint32_t mixed = hash ^ address ^ (uint32_t)(value & 0x3fffu);
    return mixed * 0x01000193u;
}

static uint8_t verify_range(void *ctx, uint32_t base_word_address, uint16_t word_count,
                            uint32_t expected_digest) {
    uint16_t i;
    uint32_t digest = 0x811c9dc5u;
    (void)ctx;
    icsp_load_pc(base_word_address);
    for (i = 0u; i < word_count; i++) {
        digest = digest_word(digest, base_word_address + i,
                             icsp_read_word(i + 1u < word_count ? 1u : 0u));
    }
    return digest == expected_digest ? 1u : 0u;
}

static uint8_t read_words(void *ctx, uint32_t base_word_address, uint16_t word_count,
                          uint8_t *out) {
    uint16_t i;
    (void)ctx;
    if (word_count > (PB_MAX_PAYLOAD_SIZE / 2u) || out == NULL) {
        return 0u;
    }
    icsp_load_pc(base_word_address);
    for (i = 0u; i < word_count; i++) {
        uint16_t word = icsp_read_word(i + 1u < word_count ? 1u : 0u);
        out[(uint16_t)i * 2u] = (uint8_t)(word & 0xffu);
        out[((uint16_t)i * 2u) + 1u] = (uint8_t)(word >> 8);
    }
    return 1u;
}

static void run_target(void *ctx) {
    (void)ctx;
    icsp_data_input();
#if defined(__XC8)
    TRISCbits.TRISC5 = 1u;
#endif
    target_uart_init();
    reset_target(ctx);
}

static void usb_bd_set_addr(uint8_t index, uint16_t addr) {
    g_bdt[index].adrl = (uint8_t)(addr & 0xffu);
    g_bdt[index].adrh = (uint8_t)(addr >> 8);
}

static void usb_arm_out(uint8_t ep, uint8_t count, uint8_t *toggle) {
    uint8_t index = USB_BDT_INDEX(ep, USB_DIR_OUT);
    g_bdt[index].cnt = count;
    g_bdt[index].stat = USB_BD_UOWN | USB_BD_DTSEN | (*toggle != 0u ? USB_BD_DTS : 0u);
    *toggle ^= 1u;
}

static void usb_arm_in(uint8_t ep, uint8_t count, uint8_t *toggle) {
    uint8_t index = USB_BDT_INDEX(ep, USB_DIR_IN);
    g_bdt[index].cnt = count;
    g_bdt[index].stat = USB_BD_UOWN | USB_BD_DTSEN | (*toggle != 0u ? USB_BD_DTS : 0u);
    *toggle ^= 1u;
}

static void usb_copy_to_ep0(const uint8_t *src, uint8_t len) {
    uint8_t i;
    for (i = 0u; i < len; i++) {
        g_ep0_in[i] = src[i];
    }
}

static void usb_send_ep0_zlp(void) {
    g_ctrl_data = NULL;
    g_ctrl_remaining = 0u;
    usb_arm_in(USB_EP0, 0u, &g_ep0_in_dts);
}

static void usb_send_ep0_next(void) {
    uint8_t len = PB_USB_EP0_SIZE;
    if (g_ctrl_remaining < len) {
        len = (uint8_t)g_ctrl_remaining;
    }
    if (len > 0u && g_ctrl_data != NULL) {
        usb_copy_to_ep0(g_ctrl_data, len);
        g_ctrl_data += len;
        g_ctrl_remaining = (uint16_t)(g_ctrl_remaining - len);
    }
    usb_arm_in(USB_EP0, len, &g_ep0_in_dts);
}

static void usb_send_control_data(const uint8_t *data, uint16_t available_len,
                                  uint16_t requested_len) {
    g_ctrl_data = data;
    g_ctrl_remaining = available_len < requested_len ? available_len : requested_len;
    usb_send_ep0_next();
}

static void usb_stall_ep0(void) {
    g_bdt[USB_BDT_INDEX(USB_EP0, USB_DIR_IN)].stat = USB_BD_UOWN | USB_BD_BSTALL;
    g_bdt[USB_BDT_INDEX(USB_EP0, USB_DIR_OUT)].stat = USB_BD_UOWN | USB_BD_BSTALL;
#if defined(__XC8)
    UEP0bits.EPSTALL = 1u;
#endif
}

static uint16_t setup_w_value(void) {
    return (uint16_t)g_ep0_out[2] | ((uint16_t)g_ep0_out[3] << 8);
}

static uint16_t setup_w_index(void) {
    return (uint16_t)g_ep0_out[4] | ((uint16_t)g_ep0_out[5] << 8);
}

static uint16_t setup_w_length(void) {
    return (uint16_t)g_ep0_out[6] | ((uint16_t)g_ep0_out[7] << 8);
}

static void usb_handle_get_descriptor(uint16_t value, uint16_t index, uint16_t length) {
    uint8_t descriptor_type = (uint8_t)(value >> 8);
    uint8_t descriptor_index = (uint8_t)value;
    (void)index;
    switch (descriptor_type) {
    case USB_DESC_DEVICE:
        usb_send_control_data(pb_usb_device_descriptor, sizeof(pb_usb_device_descriptor), length);
        break;
    case USB_DESC_CONFIGURATION:
        usb_send_control_data(pb_usb_config_descriptor, sizeof(pb_usb_config_descriptor), length);
        break;
    case USB_DESC_STRING:
        if (descriptor_index == 0u) {
            usb_send_control_data(g_string0, sizeof(g_string0), length);
        } else if (descriptor_index == 1u) {
            usb_send_control_data(g_string_manufacturer, sizeof(g_string_manufacturer), length);
        } else if (descriptor_index == 2u) {
            usb_send_control_data(g_string_product, sizeof(g_string_product), length);
        } else if (descriptor_index == 3u) {
            usb_send_control_data(g_string_serial, sizeof(g_string_serial), length);
        } else {
            usb_stall_ep0();
        }
        break;
    case USB_DESC_HID:
        usb_send_control_data(&pb_usb_config_descriptor[USB_HID_DESCRIPTOR_OFFSET], 9u, length);
        break;
    case USB_DESC_HID_REPORT:
        usb_send_control_data(pb_usb_hid_report_descriptor, sizeof(pb_usb_hid_report_descriptor),
                              length);
        break;
    default:
        usb_stall_ep0();
        break;
    }
}

static void usb_configure_endpoints(void) {
#if defined(__XC8)
    UEP1 = USB_UEP_DATA_BIDIR;
    UEP2 = USB_UEP_DATA_BIDIR;
    UEP3 = USB_UEP_DATA_IN;
#endif
    g_hid_out_dts = 0u;
    g_hid_in_dts = 0u;
    g_cdc_out_dts = 0u;
    g_cdc_in_dts = 0u;
    usb_arm_out(USB_EP_HID, PB_USB_HID_EP_SIZE, &g_hid_out_dts);
    usb_arm_out(USB_EP_CDC, PB_USB_CDC_EP_SIZE, &g_cdc_out_dts);
}

static uint8_t usb_handle_setup(void) {
    uint8_t request_type = g_ep0_out[0];
    uint8_t request = g_ep0_out[1];
    uint16_t value = setup_w_value();
    uint16_t index = setup_w_index();
    uint16_t length = setup_w_length();
    uint8_t zero[2] = {0u, 0u};
    uint8_t iface = 0u;
    uint8_t report[PB_USB_HID_EP_SIZE];
    uint8_t caller_rearm_out = 1u;

    g_ep0_in_dts = 1u;
    g_ep0_out_dts = 1u;
    g_ep0_out_data_stage = 0u;
#if defined(__XC8)
    UEP0bits.EPSTALL = 0u;
#endif

    if ((request_type & 0x60u) == 0x00u) {
        switch (request) {
        case USB_REQ_GET_STATUS:
            usb_send_control_data(zero, sizeof(zero), length);
            break;
        case USB_REQ_CLEAR_FEATURE:
        case USB_REQ_SET_FEATURE:
            usb_send_ep0_zlp();
            break;
        case USB_REQ_SET_ADDRESS:
            g_pending_address = (uint8_t)(value & 0x7fu);
            g_address_pending = 1u;
            usb_send_ep0_zlp();
            break;
        case USB_REQ_GET_DESCRIPTOR:
            usb_handle_get_descriptor(value, index, length);
            break;
        case USB_REQ_GET_CONFIGURATION:
            usb_send_control_data(&g_configuration, 1u, length);
            break;
        case USB_REQ_SET_CONFIGURATION:
            g_configuration = (uint8_t)(value & 0xffu);
            usb_configure_endpoints();
            usb_send_ep0_zlp();
            break;
        case USB_REQ_GET_INTERFACE:
            usb_send_control_data(&iface, 1u, length);
            break;
        case USB_REQ_SET_INTERFACE:
            usb_send_ep0_zlp();
            break;
        default:
            usb_stall_ep0();
            break;
        }
    } else if ((request_type & 0x7fu) == 0x21u && index == 0u) {
        switch (request) {
        case CDC_REQ_SET_LINE_CODING:
            g_ep0_out_data_stage = CDC_REQ_SET_LINE_CODING;
            usb_arm_out(USB_EP0, PB_USB_EP0_SIZE, &g_ep0_out_dts);
            caller_rearm_out = 0u;
            break;
        case CDC_REQ_GET_LINE_CODING:
            usb_send_control_data(g_cdc_line_coding, sizeof(g_cdc_line_coding), length);
            break;
        case CDC_REQ_SET_CONTROL_LINE_STATE:
            usb_send_ep0_zlp();
            break;
        default:
            usb_stall_ep0();
            break;
        }
    } else if ((request_type & 0x7fu) == 0x21u && index == 2u) {
        if (request == HID_REQ_SET_IDLE) {
            usb_send_ep0_zlp();
        } else if (request == HID_REQ_GET_REPORT) {
            memset(report, 0, sizeof(report));
            usb_send_control_data(report, sizeof(report), length);
        } else {
            usb_stall_ep0();
        }
    } else {
        usb_stall_ep0();
    }
    return caller_rearm_out;
}

static void usb_reset(void) {
    uint8_t i;
    for (i = 0u; i < USB_BDT_ENTRY_COUNT; i++) {
        g_bdt[i].stat = 0u;
        g_bdt[i].cnt = 0u;
    }
    usb_bd_set_addr(USB_BDT_INDEX(USB_EP0, USB_DIR_OUT), USB_ADDR_EP0_OUT);
    usb_bd_set_addr(USB_BDT_INDEX(USB_EP0, USB_DIR_IN), USB_ADDR_EP0_IN);
    usb_bd_set_addr(USB_BDT_INDEX(USB_EP_HID, USB_DIR_OUT), USB_ADDR_HID_OUT);
    usb_bd_set_addr(USB_BDT_INDEX(USB_EP_HID, USB_DIR_IN), USB_ADDR_HID_IN);
    usb_bd_set_addr(USB_BDT_INDEX(USB_EP_CDC, USB_DIR_OUT), USB_ADDR_CDC_OUT);
    usb_bd_set_addr(USB_BDT_INDEX(USB_EP_CDC, USB_DIR_IN), USB_ADDR_CDC_IN);
    usb_bd_set_addr(USB_BDT_INDEX(USB_EP_CDC_NOTIFY, USB_DIR_IN), USB_ADDR_CDC_NOTIFY_IN);

    g_configuration = 0u;
    g_address_pending = 0u;
    g_pending_address = 0u;
    g_ep0_in_dts = 0u;
    g_ep0_out_dts = 0u;
    g_ep0_out_data_stage = 0u;
    g_cdc_control_len = 0u;

#if defined(__XC8)
    UADDR = 0u;
    UEP0 = USB_UEP_CONTROL_BIDIR;
    UEP1 = 0u;
    UEP2 = 0u;
    UEP3 = 0u;
    UIR = 0u;
#endif
    usb_arm_out(USB_EP0, PB_USB_EP0_SIZE, &g_ep0_out_dts);
}

static void usb_init(void) {
#if defined(__XC8)
    UCON = 0u;
    UCFG = 0x14u;
    UIE = 0u;
    UEIE = 0u;
#endif
    usb_reset();
#if defined(__XC8)
    UCON = 0u;
    while (UCONbits.USBEN == 0u) {
        UCONbits.USBEN = 1u;
    }
#endif
}

static void usb_handle_ep0_out(void) {
    if (g_ep0_out_data_stage == CDC_REQ_SET_LINE_CODING) {
        g_ep0_out_data_stage = 0u;
        usb_send_ep0_zlp();
        usb_arm_out(USB_EP0, PB_USB_EP0_SIZE, &g_ep0_out_dts);
    } else if (g_bdt[USB_BDT_INDEX(USB_EP0, USB_DIR_OUT)].cnt == PB_USB_EP0_SIZE) {
        if (usb_handle_setup() != 0u) {
            usb_arm_out(USB_EP0, PB_USB_EP0_SIZE, &g_ep0_out_dts);
        }
    } else {
        usb_arm_out(USB_EP0, PB_USB_EP0_SIZE, &g_ep0_out_dts);
    }
#if defined(__XC8)
    UCONbits.PKTDIS = 0u;
#endif
}

static void usb_handle_ep0_in(void) {
    if (g_address_pending != 0u) {
#if defined(__XC8)
        UADDR = g_pending_address;
#endif
        g_address_pending = 0u;
    } else if (g_ctrl_remaining > 0u) {
        usb_send_ep0_next();
    }
}

static void usb_handle_hid_out(const pb_hal_t *hal) {
    pb_handle_request(&g_state, hal, (const uint8_t *)g_hid_out, (uint8_t *)g_hid_in);
    usb_arm_in(USB_EP_HID, PB_USB_HID_EP_SIZE, &g_hid_in_dts);
    usb_arm_out(USB_EP_HID, PB_USB_HID_EP_SIZE, &g_hid_out_dts);
}

static void usb_arm_cdc_control_response(void) {
    uint8_t len = (uint8_t)(4u + g_cdc_in[3]);
    if (len > PB_HID_PACKET_SIZE) {
        len = PB_HID_PACKET_SIZE;
    }
    usb_arm_in(USB_EP_CDC, len, &g_cdc_in_dts);
}

static void usb_handle_cdc_out(const pb_hal_t *hal) {
    uint8_t len = g_bdt[USB_BDT_INDEX(USB_EP_CDC, USB_DIR_OUT)].cnt;
    uint8_t i;
    if (len == PB_HID_PACKET_SIZE && g_cdc_out[0] == PB_MAGIC) {
        pb_handle_request(&g_state, hal, (const uint8_t *)g_cdc_out, (uint8_t *)g_cdc_in);
        usb_arm_cdc_control_response();
        usb_arm_out(USB_EP_CDC, PB_USB_CDC_EP_SIZE, &g_cdc_out_dts);
        return;
    }
    if (g_state.mode != PB_MODE_RUNNING) {
        for (i = 0u; i < len; i++) {
            if (g_cdc_control_len == 0u && g_cdc_out[i] != PB_MAGIC) {
                continue;
            }
            g_cdc_control_packet[g_cdc_control_len++] = g_cdc_out[i];
            if (g_cdc_control_len == PB_HID_PACKET_SIZE) {
                pb_handle_request(&g_state, hal, g_cdc_control_packet, (uint8_t *)g_cdc_in);
                usb_arm_cdc_control_response();
                g_cdc_control_len = 0u;
            }
        }
    } else {
        for (i = 0u; i < len; i++) {
#if defined(__XC8)
            while (PIR1bits.TXIF == 0u) {
            }
            TXREG = g_cdc_out[i];
#endif
        }
    }
    usb_arm_out(USB_EP_CDC, PB_USB_CDC_EP_SIZE, &g_cdc_out_dts);
}

static void usb_service_uart_to_cdc(void) {
    uint8_t len = 0u;
    uint8_t byte;
    if (g_configuration == 0u || g_state.mode != PB_MODE_RUNNING ||
        (g_bdt[USB_BDT_INDEX(USB_EP_CDC, USB_DIR_IN)].stat & USB_BD_UOWN) != 0u) {
        return;
    }
    while (len < PB_USB_CDC_EP_SIZE && uart_rx_pop(&byte) != 0u) {
        g_cdc_in[len++] = byte;
    }
    if (len > 0u) {
        usb_arm_in(USB_EP_CDC, len, &g_cdc_in_dts);
    }
}

static void usb_service_uart_rx(void) {
    if (g_configuration == 0u || g_state.mode != PB_MODE_RUNNING) {
        return;
    }
#if defined(__XC8)
    if (RCSTAbits.OERR != 0u) {
        RCSTAbits.CREN = 0u;
        RCSTAbits.CREN = 1u;
    }
    while (PIR1bits.RCIF != 0u) {
        uart_rx_push(RCREG);
    }
#endif
}

static void usb_service(const pb_hal_t *hal) {
#if defined(__XC8)
    if (UIRbits.URSTIF != 0u) {
        UIRbits.URSTIF = 0u;
        usb_reset();
    }
    if (UIRbits.UERRIF != 0u) {
        UEIR = 0u;
        UIRbits.UERRIF = 0u;
    }
    while (UIRbits.TRNIF != 0u) {
        uint8_t stat = USTAT;
        uint8_t ep = (uint8_t)((stat >> 3) & 0x0fu);
        uint8_t dir = (stat & 0x04u) != 0u ? USB_DIR_IN : USB_DIR_OUT;
        UIRbits.TRNIF = 0u;
        if (ep == USB_EP0 && dir == USB_DIR_OUT) {
            usb_handle_ep0_out();
        } else if (ep == USB_EP0 && dir == USB_DIR_IN) {
            usb_handle_ep0_in();
        } else if (ep == USB_EP_HID && dir == USB_DIR_OUT) {
            usb_handle_hid_out(hal);
        } else if (ep == USB_EP_CDC && dir == USB_DIR_OUT) {
            usb_handle_cdc_out(hal);
        }
    }
#else
    (void)hal;
#endif
    usb_service_uart_rx();
    usb_service_uart_to_cdc();
}

int main(void) {
    const pb_hal_t hal = {
        reset_target, enter_programming, read_target_id, erase_row,  write_chunk,
        commit_row,   verify_range,      read_words,     run_target, 0,
    };

    board_init();
    pb_state_init(&g_state);
    usb_init();

    for (;;) {
        usb_service(&hal);
    }
}
