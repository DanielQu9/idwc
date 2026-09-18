#include <stdio.h>
#include <stdbool.h>
#include <stdint.h>
#include <inttypes.h>
#include <stdlib.h>

static void idwc_fail(const char *message) {
    fflush(stdout);
    fputs(message, stderr);
    exit(101);
}

static int32_t idwc_checked(int64_t value) {
    if (value < INT32_MIN || value > INT32_MAX) {
        idwc_fail("idwc: integer overflow\n");
    }
    return (int32_t)value;
}

static int32_t idwc_add(int32_t a, int32_t b) {
    return idwc_checked((int64_t)a + (int64_t)b);
}

static int32_t idwc_div(int32_t a, int32_t b) {
    if (b == 0) {
        idwc_fail("idwc: division by zero\n");
    }
    if (a == INT32_MIN && b == -1) {
        idwc_fail("idwc: integer overflow\n");
    }
    return a / b;
}

static int32_t idwc_mul(int32_t a, int32_t b) {
    return idwc_checked((int64_t)a * (int64_t)b);
}

int main(void) {
    int32_t idwc_v0 = INT32_C(6);
    (void)idwc_v0;
    const int32_t idwc_t0 = idwc_mul(INT32_C(3), INT32_C(4));
    const int32_t idwc_t1 = idwc_add(idwc_v0, idwc_t0);
    idwc_v0 = idwc_t1;
    const bool idwc_v1 = false;
    (void)idwc_v1;
    const bool idwc_t2 = (idwc_v0 >= INT32_C(18));
    bool idwc_t3 = idwc_t2;
    if (idwc_t3) {
        const bool idwc_t4 = (!idwc_v1);
        idwc_t3 = idwc_t4;
    }
    const bool idwc_v2 = idwc_t3;
    (void)idwc_v2;
    const int32_t idwc_t5 = idwc_v0;
    const bool idwc_t6 = idwc_v2;
    fputs("total = ", stdout);
    printf("%" PRId32, idwc_t5);
    fputs(", ready = ", stdout);
    fputs(idwc_t6 ? "true" : "false", stdout);
    putchar('\n');
    {
        const int32_t idwc_t7 = idwc_div(idwc_v0, INT32_C(2));
        const int32_t idwc_v3 = idwc_t7;
        (void)idwc_v3;
        const int32_t idwc_t8 = idwc_v3;
        fputs("inner total = ", stdout);
        printf("%" PRId32, idwc_t8);
        putchar('\n');
    }
    idwc_v0 = INT32_C(17);
    const int32_t idwc_t9 = idwc_v0;
    fputs("outer total = ", stdout);
    printf("%" PRId32, idwc_t9);
    putchar('\n');
    return 0;
}
