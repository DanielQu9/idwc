#include <stdio.h>
#include <stdbool.h>
#include <stdint.h>
#include <inttypes.h>
#include <stdlib.h>
#include <signal.h>

static void idwc_fail(const char *message) {
#ifdef SIGPIPE
    signal(SIGPIPE, SIG_IGN);
#endif
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

static int32_t idwc_rem(int32_t a, int32_t b) {
    if (b == 0) {
        idwc_fail("idwc: division by zero\n");
    }
    if (a == INT32_MIN && b == -1) {
        idwc_fail("idwc: integer overflow\n");
    }
    return a % b;
}

static int32_t idwc_sub(int32_t a, int32_t b) {
    return idwc_checked((int64_t)a - (int64_t)b);
}

int main(void) {
    int32_t idwc_v0 = INT32_C(0);
    (void)idwc_v0;
    int32_t idwc_v1 = INT32_C(0);
    (void)idwc_v1;
    for (;;) {
        const bool idwc_t0 = (idwc_v0 < INT32_C(6));
        if (!(idwc_t0)) {
            break;
        }
        const int32_t idwc_t1 = idwc_add(idwc_v0, INT32_C(1));
        idwc_v0 = idwc_t1;
        const int32_t idwc_t2 = idwc_rem(idwc_v0, INT32_C(2));
        const bool idwc_t3 = (idwc_t2 == INT32_C(0));
        if (idwc_t3) {
            continue;
        }
        const int32_t idwc_t4 = idwc_add(idwc_v1, idwc_v0);
        idwc_v1 = idwc_t4;
    }
    const bool idwc_t5 = (idwc_v1 > INT32_C(9));
    if (idwc_t5) {
        const int32_t idwc_t6 = idwc_v1;
        fputs("large total: ", stdout);
        printf("%" PRId32, idwc_t6);
        putchar('\n');
    } else {
        const bool idwc_t7 = (idwc_v1 == INT32_C(9));
        if (idwc_t7) {
            const int32_t idwc_t8 = idwc_v1;
            fputs("total = ", stdout);
            printf("%" PRId32, idwc_t8);
            putchar('\n');
        } else {
            const int32_t idwc_t9 = idwc_v1;
            fputs("small total: ", stdout);
            printf("%" PRId32, idwc_t9);
            putchar('\n');
        }
    }
    for (;;) {
        const int32_t idwc_t10 = idwc_sub(idwc_v1, INT32_C(1));
        idwc_v1 = idwc_t10;
        const bool idwc_t11 = (idwc_v1 == INT32_C(6));
        if (idwc_t11) {
            break;
        }
    }
    const int32_t idwc_t12 = idwc_v1;
    fputs("after loop = ", stdout);
    printf("%" PRId32, idwc_t12);
    putchar('\n');
    return 0;
}
