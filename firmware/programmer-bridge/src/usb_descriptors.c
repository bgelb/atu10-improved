#include "usb_descriptors.h"

const uint8_t pb_usb_device_descriptor[18] = {
    18,   /* bLength */
    0x01, /* DEVICE */
    0x00,
    0x02, /* USB 2.0 */
    0xEF, /* Miscellaneous device class for composite/IAD */
    0x02, /* Common class */
    0x01, /* Interface association descriptor */
    PB_USB_EP0_SIZE,
    (uint8_t)(PB_USB_VENDOR_ID & 0xffu),
    (uint8_t)(PB_USB_VENDOR_ID >> 8),
    (uint8_t)(PB_USB_PRODUCT_ID & 0xffu),
    (uint8_t)(PB_USB_PRODUCT_ID >> 8),
    0x00,
    0x01, /* bcdDevice */
    1,    /* manufacturer string */
    2,    /* product string */
    3,    /* serial string */
    1,    /* one configuration */
};

const uint8_t pb_usb_hid_report_descriptor[34] = {
    0x06, 0x00, 0xff, /* Usage Page: vendor */
    0x09, 0x01,       /* Usage */
    0xa1, 0x01,       /* Collection: application */
    0x15, 0x00,       /* Logical min */
    0x26, 0xff, 0x00, /* Logical max */
    0x75, 0x08,       /* Report size */
    0x95, 0x40,       /* Report count */
    0x09, 0x01,       /* Usage */
    0x81, 0x02,       /* Input */
    0x95, 0x40,       /* Report count */
    0x09, 0x01,       /* Usage */
    0x91, 0x02,       /* Output */
    0xc0,             /* End collection */
    0x00, 0x00, 0x00, 0x00, 0x00,
};

const uint8_t pb_usb_config_descriptor[PB_USB_CONFIG_TOTAL_LEN] = {
    9,    /* bLength */
    0x02, /* CONFIGURATION */
    (uint8_t)(PB_USB_CONFIG_TOTAL_LEN & 0xffu),
    (uint8_t)(PB_USB_CONFIG_TOTAL_LEN >> 8),
    3,    /* interfaces: CDC control, CDC data, HID */
    1,    /* configuration value */
    0,    /* no string */
    0x80, /* bus powered */
    50,   /* 100 mA */

    8,    /* IAD length */
    0x0b, /* IAD */
    0,    /* first interface */
    2,    /* interface count */
    0x02, /* CDC */
    0x02, /* ACM */
    0x01, /* AT commands */
    0,

    9, /* CDC communication interface */
    0x04,
    0,
    0,
    1, /* CDC notification endpoint */
    0x02,
    0x02,
    0x01,
    0,

    5, /* CDC header functional descriptor */
    0x24,
    0x00,
    0x10,
    0x01,

    4, /* CDC ACM functional descriptor */
    0x24,
    0x02,
    0x02,

    5, /* CDC union functional descriptor */
    0x24,
    0x06,
    0,
    1,

    5, /* CDC call management descriptor */
    0x24,
    0x01,
    0,
    1,

    7, /* CDC notification IN EP3 */
    0x05,
    0x83,
    0x03,
    PB_USB_CDC_NOTIFY_EP_SIZE,
    0,
    16,

    9, /* CDC data interface */
    0x04,
    1,
    0,
    2,
    0x0a,
    0,
    0,
    0,

    7, /* CDC data OUT EP2 */
    0x05,
    0x02,
    0x02,
    PB_USB_CDC_EP_SIZE,
    0,
    0,

    7, /* CDC data IN EP2 */
    0x05,
    0x82,
    0x02,
    PB_USB_CDC_EP_SIZE,
    0,
    0,

    9, /* HID interface */
    0x04,
    2,
    0,
    2,
    0x03,
    0,
    0,
    0,

    9, /* HID descriptor */
    0x21,
    0x11,
    0x01,
    0,
    1,
    0x22,
    sizeof(pb_usb_hid_report_descriptor),
    0,

    7, /* HID OUT EP1 */
    0x05,
    0x01,
    0x03,
    PB_USB_HID_EP_SIZE,
    0,
    1,

    7, /* HID IN EP1 */
    0x05,
    0x81,
    0x03,
    PB_USB_HID_EP_SIZE,
    0,
    1,
};
