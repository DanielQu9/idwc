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
    const int32_t idwc_t0 = INT32_C(12);
    const int32_t idwc_t1 = INT32_C(18);
    const int32_t idwc_t2 = INT32_C(9);
    const int32_t idwc_t3 = INT32_C(15);
    int32_t idwc_v0[4] = {idwc_t0, idwc_t1, idwc_t2, idwc_t3};
    (void)idwc_v0;
    const int32_t idwc_v1[4] = {idwc_v0[0], idwc_v0[1], idwc_v0[2], idwc_v0[3]};
    (void)idwc_v1;
    size_t idwc_v2 = ((size_t)UINT64_C(0));
    (void)idwc_v2;
    for (;;) {
        const bool idwc_t4 = (idwc_v2 < ((size_t)UINT64_C(4)));
        if (!(idwc_t4)) {
            break;
        }
        const size_t idwc_t5 = idwc_v2;
        if (idwc_t5 >= ((size_t)UINT64_C(4))) { idwc_fail("idwc: array index out of bounds\n"); }
        idwc_v0[idwc_t5] = idwc_add(idwc_v0[idwc_t5], INT32_C(1));
        const size_t idwc_t6 = idwc_uadd(idwc_v2, ((size_t)UINT64_C(1)));
        idwc_v2 = idwc_t6;
    }
    const size_t idwc_t7 = ((size_t)UINT64_C(0));
    if (idwc_t7 >= ((size_t)UINT64_C(4))) { idwc_fail("idwc: array index out of bounds\n"); }
    const int32_t idwc_t8 = idwc_v0[idwc_t7];
    const size_t idwc_t9 = ((size_t)UINT64_C(3));
    if (idwc_t9 >= ((size_t)UINT64_C(4))) { idwc_fail("idwc: array index out of bounds\n"); }
    const int32_t idwc_t10 = idwc_v0[idwc_t9];
    const size_t idwc_t11 = ((size_t)UINT64_C(0));
    if (idwc_t11 >= ((size_t)UINT64_C(4))) { idwc_fail("idwc: array index out of bounds\n"); }
    const int32_t idwc_t12 = idwc_v1[idwc_t11];
    const size_t idwc_t13 = ((size_t)UINT64_C(4));
    fputs("first = ", stdout);
    printf("%" PRId32, idwc_t8);
    fputs(", last = ", stdout);
    printf("%" PRId32, idwc_t10);
    fputs(", original first = ", stdout);
    printf("%" PRId32, idwc_t12);
    fputs(", length = ", stdout);
    printf("%zu", idwc_t13);
    putchar('\n');
    fputs("scores = ", stdout);
    putchar('[');
    for (size_t idwc_t14 = 0; idwc_t14 < 4; ++idwc_t14) {
        if (idwc_t14 != 0) { fputs(", ", stdout); }
        printf("%" PRId32, idwc_v0[idwc_t14]);
    }
    putchar(']');
    putchar('\n');
    return 0;
}
