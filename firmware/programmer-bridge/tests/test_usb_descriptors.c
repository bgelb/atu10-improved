#include "unity.h"
#include "usb_descriptors.h"

static void test_descriptor_lengths_match_declared_sizes(void) {
    TEST_ASSERT_EQUAL_INT(18, sizeof(pb_usb_device_descriptor));
    TEST_ASSERT_EQUAL_INT(PB_USB_CONFIG_TOTAL_LEN, sizeof(pb_usb_config_descriptor));
    TEST_ASSERT_EQUAL_INT(34, sizeof(pb_usb_hid_report_descriptor));
}

static void test_configuration_descriptor_total_length_is_consistent(void) {
    uint16_t total =
        ((uint16_t)pb_usb_config_descriptor[3] << 8) | (uint16_t)pb_usb_config_descriptor[2];
    TEST_ASSERT_EQUAL_INT(PB_USB_CONFIG_TOTAL_LEN, total);
}

static void test_endpoint_budget_stays_within_pic16f1454_limits(void) {
    TEST_ASSERT_EQUAL_UINT8(0x02, pb_usb_config_descriptor[56]);
    TEST_ASSERT_EQUAL_UINT8(0x82, pb_usb_config_descriptor[63]);
    TEST_ASSERT_EQUAL_UINT8(0x01, pb_usb_config_descriptor[88]);
    TEST_ASSERT_EQUAL_UINT8(0x81, pb_usb_config_descriptor[95]);
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_descriptor_lengths_match_declared_sizes);
    RUN_TEST(test_configuration_descriptor_total_length_is_consistent);
    RUN_TEST(test_endpoint_budget_stays_within_pic16f1454_limits);
    return UNITY_END();
}
