#ifndef ATU10_PROGRAMMER_BRIDGE_USB_DESCRIPTORS_H
#define ATU10_PROGRAMMER_BRIDGE_USB_DESCRIPTORS_H

#include <stdint.h>

#define PB_USB_VENDOR_ID 0x1209u
#define PB_USB_PRODUCT_ID 0xA710u
#define PB_USB_EP0_SIZE 8u
#define PB_USB_HID_EP_SIZE 64u
#define PB_USB_CDC_EP_SIZE 64u
#define PB_USB_CONFIG_TOTAL_LEN 100u

extern const uint8_t pb_usb_device_descriptor[18];
extern const uint8_t pb_usb_config_descriptor[PB_USB_CONFIG_TOTAL_LEN];
extern const uint8_t pb_usb_hid_report_descriptor[34];

#endif
