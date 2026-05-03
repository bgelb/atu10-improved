#include "unity.h"

int unity_failures;

void unity_assert_true(int condition, const char *expr, const char *file, int line) {
    if (!condition) {
        printf("%s:%d: assertion failed: %s\n", file, line, expr);
        unity_failures++;
    }
}

void unity_assert_int(int expected, int actual, const char *expr, const char *file, int line) {
    if (expected != actual) {
        printf("%s:%d: assertion failed: %s expected %d got %d\n", file, line, expr, expected,
               actual);
        unity_failures++;
    }
}

void unity_assert_memory(const void *expected, const void *actual, unsigned long len,
                         const char *expr, const char *file, int line) {
    if (memcmp(expected, actual, len) != 0) {
        printf("%s:%d: assertion failed: %s memory differs\n", file, line, expr);
        unity_failures++;
    }
}

void unity_assert_string(const char *expected, const char *actual, const char *expr,
                         const char *file, int line) {
    if (strcmp(expected, actual) != 0) {
        printf("%s:%d: assertion failed: %s expected '%s' got '%s'\n", file, line, expr, expected,
               actual);
        unity_failures++;
    }
}
