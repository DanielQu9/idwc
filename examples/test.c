#include <stdio.h>
#include <stdbool.h>
#include <stdint.h>
#include <inttypes.h>
#include <stdlib.h>
#include <signal.h>
#include <float.h>
#include <math.h>
#include <fenv.h>
#include <locale.h>
#include <string.h>

static void idwc_fail(const char *message) {
#ifdef SIGPIPE
    signal(SIGPIPE, SIG_IGN);
#endif
    fflush(stdout);
    fputs(message, stderr);
    exit(101);
}

/* ASCII whitespace is independent of the host locale. */
static bool idwc_space(int byte) {
    return byte == ' ' || byte == '\t' || byte == '\n' || byte == '\r'
        || byte == '\v' || byte == '\f';
}

/* Caller supplies 129 bytes: 128 token bytes plus a terminating NUL. */
static size_t idwc_read_token(unsigned char token[129]) {
    size_t length = 0;
    int byte;
    do {
        byte = fgetc(stdin);
        if (byte == EOF) {
            idwc_fail(ferror(stdin) ? "idwc: stdin I/O error\n" : "idwc: unexpected EOF\n");
        }
    } while (idwc_space(byte));
    for (;;) {
        if (length == 128) {
            idwc_fail("idwc: input token too long\n");
        }
        token[length++] = (unsigned char)byte;
        byte = fgetc(stdin);
        if (byte == EOF) {
            if (ferror(stdin)) {
                idwc_fail("idwc: stdin I/O error\n");
            }
            break;
        }
        if (idwc_space(byte)) { break; }
    }
    token[length] = '\0';
    return length;
}

/* Only binary64 with one rounded result per operation is accepted. */
#if defined(__FAST_MATH__) || (defined(__FINITE_MATH_ONLY__) && __FINITE_MATH_ONLY__ > 0)
#error "idwc f64 requires fast-math to be disabled"
#endif
_Static_assert(FLT_RADIX == 2 && DBL_MANT_DIG == 53 && DBL_MAX_EXP == 1024
    && DBL_MIN_EXP == -1021 && sizeof(double) == sizeof(uint64_t),
    "idwc f64 requires IEEE binary64 double");
_Static_assert(FLT_EVAL_METHOD == 0, "idwc f64 rejects excess floating-point precision");

/* Establish a nontrapping, nearest-even environment and the C decimal locale. */
static void idwc_float_init(void) {
    double one = 1.0;
    uint64_t bits;
    memcpy(&bits, &one, sizeof bits);
    if (bits != UINT64_C(0x3ff0000000000000)
        || setlocale(LC_NUMERIC, "C") == NULL
        || fesetenv(FE_DFL_ENV) != 0 || fesetround(FE_TONEAREST) != 0) {
        idwc_fail("idwc: unsupported floating-point environment\n");
    }
    volatile double minimum = DBL_MIN;
    volatile double tiny = DBL_TRUE_MIN;
    if (minimum / 2.0 == 0.0 || tiny * 1.0 == 0.0) {
        idwc_fail("idwc: unsupported floating-point environment\n");
    }
}

static double idwc_fdiv(double a, double b) {
    if (b == 0.0) {
        if (a == 0.0 || isnan(a)) { return NAN; }
        return signbit(a) != signbit(b) ? -INFINITY : INFINITY;
    }
    return a / b;
}

/* Validate the whole decimal token before delegating binary64 rounding to strtod. */
static double idwc_read_f64(void) {
    unsigned char token[129];
    size_t length = idwc_read_token(token);
    if (length == 3 && memcmp(token, "NaN", 3) == 0) { return NAN; }
    if (length == 3 && memcmp(token, "inf", 3) == 0) { return INFINITY; }
    if (length == 4 && memcmp(token, "+inf", 4) == 0) { return INFINITY; }
    if (length == 4 && memcmp(token, "-inf", 4) == 0) { return -INFINITY; }
    size_t index = (token[0] == '+' || token[0] == '-') ? 1 : 0;
    size_t digits = 0;
    bool nonzero = false;
    while (index < length && token[index] >= '0' && token[index] <= '9') {
        nonzero = nonzero || token[index] != '0';
        ++digits;
        ++index;
    }
    if (index < length && token[index] == '.') {
        ++index;
        while (index < length && token[index] >= '0' && token[index] <= '9') {
            nonzero = nonzero || token[index] != '0';
            ++digits;
            ++index;
        }
    }
    if (digits == 0) { idwc_fail("idwc: invalid float\n"); }
    if (index < length && (token[index] == 'e' || token[index] == 'E')) {
        ++index;
        if (index < length && (token[index] == '+' || token[index] == '-')) { ++index; }
        size_t start = index;
        while (index < length && token[index] >= '0' && token[index] <= '9') { ++index; }
        if (start == index) { idwc_fail("idwc: invalid float\n"); }
    }
    if (index != length) { idwc_fail("idwc: invalid float\n"); }
    char *end;
    double value = strtod((const char *)token, &end);
    if (end != (const char *)token + length) { idwc_fail("idwc: invalid float\n"); }
    /* ERANGE alone also covers representable subnormals, which are accepted. */
    if (!isfinite(value) || (nonzero && value == 0.0)) {
        idwc_fail("idwc: float out of range\n");
    }
    return value;
}

int main(void) {
    idwc_float_init();
    const volatile double idwc_t0 = idwc_read_f64();
    const double idwc_v0 = idwc_t0;
    (void)idwc_v0;
    const volatile double idwc_t1 = idwc_read_f64();
    const double idwc_v1 = idwc_t1;
    (void)idwc_v1;
    const volatile double idwc_t2 = (idwc_v1 * 0x147ae147ae147bp-59);
    const volatile double idwc_t3 = (idwc_v1 * 0x147ae147ae147bp-59);
    const volatile double idwc_t4 = (idwc_t2 * idwc_t3);
    const volatile double idwc_t5 = idwc_fdiv(idwc_v0, idwc_t4);
    const double idwc_v2 = idwc_t5;
    (void)idwc_v2;
    const bool idwc_t6 = (idwc_v2 < 0x12800000000000p-48);
    if (idwc_t6) {
        puts("\351\201\216\347\230\246");
    } else {
        const bool idwc_t7 = (idwc_v2 < 0x18000000000000p-48);
        if (idwc_t7) {
            puts("\346\250\231\346\272\226");
        } else {
            const bool idwc_t8 = (idwc_v2 < 0x1b000000000000p-48);
            if (idwc_t8) {
                puts("\351\201\216\351\207\215");
            } else {
                const bool idwc_t9 = (idwc_v2 < 0x1e000000000000p-48);
                if (idwc_t9) {
                    puts("\350\274\225\345\272\246\350\202\245\350\203\226");
                } else {
                    const bool idwc_t10 = (idwc_v2 < 0x11800000000000p-47);
                    if (idwc_t10) {
                        puts("\344\270\255\345\272\246\350\202\245\346\264\276");
                    } else {
                        puts("\351\207\215\345\272\246\350\202\245\350\203\226");
                    }
                }
            }
        }
    }
    return 0;
}
