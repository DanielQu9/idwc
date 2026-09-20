#include <stdio.h>
#include <stdbool.h>
#include <stdint.h>
#include <inttypes.h>
#include <stdlib.h>
#include <signal.h>
_Static_assert(SIZE_MAX == UINT64_MAX, "idwc usize requires matching Rust and C target widths");

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

static size_t idwc_uadd(size_t a, size_t b) {
    if (a > SIZE_MAX - b) { idwc_fail("idwc: integer overflow\n"); }
    return a + b;
}

int main(void) {
    int32_t idwc_v0 = INT32_C(0);
    (void)idwc_v0;
    {
        const int32_t idwc_t0 = INT32_C(1);
        const int32_t idwc_t1 = INT32_C(5);
        for (int32_t idwc_t2 = idwc_t0; idwc_t2 < idwc_t1; idwc_t2 = idwc_add(idwc_t2, INT32_C(1))) {
            const int32_t idwc_v1 = idwc_t2;
            (void)idwc_v1;
            const bool idwc_t3 = (idwc_v1 == INT32_C(2));
            if (idwc_t3) {
                continue;
            }
            const int32_t idwc_t4 = idwc_add(idwc_v0, idwc_v1);
            idwc_v0 = idwc_t4;
        }
    }
    {
        const size_t idwc_t5 = ((size_t)UINT64_C(0));
        const size_t idwc_t6 = ((size_t)UINT64_C(2));
        bool idwc_t8 = idwc_t5 <= idwc_t6;
        for (size_t idwc_t7 = idwc_t5; idwc_t8; idwc_t8 = idwc_t7 != idwc_t6, idwc_t7 = idwc_t8 ? idwc_uadd(idwc_t7, (size_t)UINT64_C(1)) : idwc_t7) {
            const size_t idwc_v2 = idwc_t7;
            (void)idwc_v2;
            const size_t idwc_t9 = idwc_v2;
            fputs("index = ", stdout);
            printf("%zu", idwc_t9);
            putchar('\n');
        }
    }
    puts("");
    const int32_t idwc_t10 = idwc_v0;
    fputs("total = ", stdout);
    printf("%" PRId32, idwc_t10);
    putchar('\n');
    return 0;
}
