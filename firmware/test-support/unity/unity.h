#ifndef ATU10_TEST_SUPPORT_UNITY_H
#define ATU10_TEST_SUPPORT_UNITY_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern int unity_failures;

#define UNITY_BEGIN() (unity_failures = 0)
#define UNITY_END() (unity_failures)

#define RUN_TEST(test_fn)                                                                          \
    do {                                                                                           \
        printf("RUN %s\n", #test_fn);                                                              \
        test_fn();                                                                                 \
    } while (0)

#define TEST_ASSERT_TRUE(expr) unity_assert_true((expr) != 0, #expr, __FILE__, __LINE__)
#define TEST_ASSERT_FALSE(expr) unity_assert_true((expr) == 0, #expr, __FILE__, __LINE__)
#define TEST_ASSERT_EQUAL_INT(expected, actual)                                                    \
    unity_assert_int((expected), (actual), #actual, __FILE__, __LINE__)
#define TEST_ASSERT_EQUAL_UINT8(expected, actual)                                                  \
    unity_assert_int((expected), (actual), #actual, __FILE__, __LINE__)
#define TEST_ASSERT_EQUAL_MEMORY(expected, actual, len)                                            \
    unity_assert_memory((expected), (actual), (len), #actual, __FILE__, __LINE__)
#define TEST_ASSERT_EQUAL_STRING(expected, actual)                                                 \
    unity_assert_string((expected), (actual), #actual, __FILE__, __LINE__)

void unity_assert_true(int condition, const char *expr, const char *file, int line);
void unity_assert_int(int expected, int actual, const char *expr, const char *file, int line);
void unity_assert_memory(const void *expected, const void *actual, unsigned long len,
                         const char *expr, const char *file, int line);
void unity_assert_string(const char *expected, const char *actual, const char *expr,
                         const char *file, int line);

#endif
