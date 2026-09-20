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
_Static_assert(SIZE_MAX == UINT64_MAX, "idwc usize requires matching Rust and C target widths");

static void idwc_fail(const char *message) {
#ifdef SIGPIPE
    signal(SIGPIPE, SIG_IGN);
#endif
    fflush(stdout);
    fputs(message, stderr);
    exit(101);
}

#define IDWC_VEC_LIMIT ((size_t)2048)

typedef struct { int32_t data[2048]; size_t length; } idwc_vec_i32;
typedef struct { size_t data[2048]; size_t length; } idwc_vec_usize;
typedef struct { double data[2048]; size_t length; } idwc_vec_f64;
typedef struct { bool data[2048]; size_t length; } idwc_vec_bool;

void idwc_vec_require_capacity(size_t requested) {
    if (requested > IDWC_VEC_LIMIT) {
        idwc_fail("idwc: Vec capacity exceeded\n");
    }
}

size_t idwc_vec_index(size_t length, size_t index) {
    if (index >= length) {
        idwc_fail("idwc: Vec index out of bounds\n");
    }
    return index;
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

/* Exact decimal digits, least significant first; binary64 needs at most 767. */
typedef struct {
    unsigned char digit[1100];
    int length;
    int scale;
} idwc_decimal;

/* Small-factor multiplication stays bounded (at most 5 * 9 plus carry). */
static void idwc_decimal_multiply(idwc_decimal *decimal, unsigned factor) {
    unsigned carry = 0;
    for (int i = 0; i < decimal->length; ++i) {
        unsigned value = decimal->digit[i] * factor + carry;
        decimal->digit[i] = (unsigned char)(value % 10);
        carry = value / 10;
    }
    while (carry != 0) {
        decimal->digit[decimal->length++] = (unsigned char)(carry % 10);
        carry /= 10;
    }
}

/* Decode the validated binary64 layout; x * 2^-k becomes (x * 5^k) * 10^-k. */
static idwc_decimal idwc_exact_decimal(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof bits);
    int exponent = (int)((bits >> 52) & 2047);
    uint64_t mantissa = bits & UINT64_C(0x000fffffffffffff);
    if (exponent != 0) { mantissa |= UINT64_C(0x0010000000000000); }
    exponent = exponent == 0 ? -1074 : exponent - 1075;
    idwc_decimal decimal = {{0}, 0, exponent < 0 ? -exponent : 0};
    do {
        decimal.digit[decimal.length++] = (unsigned char)(mantissa % 10);
        mantissa /= 10;
    } while (mantissa != 0);
    for (int i = 0; i < (exponent < 0 ? -exponent : exponent); ++i) {
        idwc_decimal_multiply(&decimal, exponent < 0 ? 5 : 2);
    }
    return decimal;
}

/* Decimal rounding uses nearest, ties to even, without floating-point arithmetic. */
static bool idwc_round_up(const idwc_decimal *decimal, int cut, unsigned retained) {
    if (cut <= 0 || cut > decimal->length) { return false; }
    unsigned first = decimal->digit[cut - 1];
    if (first != 5) { return first > 5; }
    for (int i = 0; i < cut - 1; ++i) {
        if (decimal->digit[i] != 0) { return true; }
    }
    return (retained & 1) != 0;
}

/* Print fixed decimal digits, preserving negative zero and adding no exponent. */
static void idwc_decimal_output(const idwc_decimal *decimal, bool negative) {
    if (negative) { putchar('-'); }
    int point = decimal->length - decimal->scale;
    if (point <= 0) { putchar('0'); }
    for (int i = decimal->length - 1; i >= decimal->scale; --i) {
        putchar('0' + decimal->digit[i]);
    }
    if (decimal->scale > 0) {
        putchar('.');
        for (int i = 0; i < -point; ++i) { putchar('0'); }
        for (int i = (point > 0 ? decimal->scale : decimal->length) - 1; i >= 0; --i) {
            putchar('0' + decimal->digit[i]);
        }
    }
}

/* Round an exact decimal to 0..18 fractional places, including carry to integers. */
static void idwc_fixed_decimal(idwc_decimal *decimal, int precision) {
    int cut = decimal->scale - precision;
    if (cut <= 0) {
        for (int i = decimal->length - 1; i >= 0; --i) { decimal->digit[i - cut] = decimal->digit[i]; }
        for (int i = 0; i < -cut; ++i) { decimal->digit[i] = 0; }
        decimal->length -= cut;
    } else {
        unsigned retained = cut < decimal->length ? decimal->digit[cut] : 0;
        bool up = idwc_round_up(decimal, cut, retained);
        int length = decimal->length - cut;
        if (length <= 0) { decimal->digit[0] = 0; length = 1; }
        else {
            for (int i = 0; i < length; ++i) { decimal->digit[i] = decimal->digit[i + cut]; }
        }
        decimal->length = length;
        for (int i = 0; up; ++i) {
            if (i == decimal->length) { decimal->digit[decimal->length++] = 0; }
            if (++decimal->digit[i] != 10) { up = false; }
            else { decimal->digit[i] = 0; }
        }
    }
    decimal->scale = precision;
}

/* Rust shortest Display breaks decimal midpoint ties towards the larger magnitude. */
static uint64_t idwc_coefficient(const idwc_decimal *decimal, int precision) {
    uint64_t coefficient = 0;
    for (int i = 0; i < precision; ++i) {
        int index = decimal->length - 1 - i;
        coefficient = coefficient * 10 + (index >= 0 ? decimal->digit[index] : 0);
    }
    return coefficient + idwc_round_up(decimal, decimal->length - precision, 1);
}

/* Compare a decimal candidate with the original value by correctly rounded strtod. */
static bool idwc_roundtrips(uint64_t coefficient, int exponent, double value) {
    char candidate[64];
    snprintf(candidate, sizeof candidate, "%" PRIu64 "e%d", coefficient, exponent);
    return strtod(candidate, NULL) == value;
}

/* Find the shortest roundtripping coefficient, then render it in Rust Display form. */
static void idwc_shortest_decimal(idwc_decimal *decimal, double value) {
    for (int precision = 1; precision <= 17; ++precision) {
        uint64_t coefficient = idwc_coefficient(decimal, precision);
        int exponent = decimal->length - decimal->scale - precision;
        if (!idwc_roundtrips(coefficient, exponent, value)) {
            /* Binary interval widths can be asymmetric at powers of two. */
            if (idwc_roundtrips(coefficient - 1, exponent, value)) { --coefficient; }
            else if (idwc_roundtrips(coefficient + 1, exponent, value)) { ++coefficient; }
            else { continue; }
        }
        while (coefficient % 10 == 0) { coefficient /= 10; ++exponent; }
        decimal->length = 0;
        do {
            decimal->digit[decimal->length++] = (unsigned char)(coefficient % 10);
            coefficient /= 10;
        } while (coefficient != 0);
        decimal->scale = exponent < 0 ? -exponent : 0;
        if (exponent > 0) {
            for (int i = decimal->length - 1; i >= 0; --i) { decimal->digit[i + exponent] = decimal->digit[i]; }
            for (int i = 0; i < exponent; ++i) { decimal->digit[i] = 0; }
            decimal->length += exponent;
        }
        return;
    }
    idwc_fail("idwc: float formatting error\n");
}

/* Rust-compatible special spelling; NaN payload and sign are intentionally ignored. */
static void idwc_print_f64(double value, int precision) {
    if (isnan(value)) { fputs("NaN", stdout); return; }
    if (isinf(value)) { fputs(signbit(value) ? "-inf" : "inf", stdout); return; }
    bool negative = signbit(value);
    if (value == 0.0 && precision < 0) { fputs(negative ? "-0" : "0", stdout); return; }
    idwc_decimal decimal = idwc_exact_decimal(value);
    if (precision >= 0) { idwc_fixed_decimal(&decimal, precision); }
    else { idwc_shortest_decimal(&decimal, negative ? -value : value); }
    idwc_decimal_output(&decimal, negative);
}
int main(void) {
    idwc_float_init();
    idwc_vec_i32 idwc_v0 = {{0}, 0};
    idwc_vec_require_capacity(3);
    const int32_t idwc_t0 = INT32_C(1);
    idwc_v0.data[idwc_v0.length++] = idwc_t0;
    const int32_t idwc_t1 = INT32_C(2);
    idwc_v0.data[idwc_v0.length++] = idwc_t1;
    const int32_t idwc_t2 = INT32_C(3);
    idwc_v0.data[idwc_v0.length++] = idwc_t2;
    (void)idwc_v0;
    const int32_t idwc_t3 = INT32_C(4);
    idwc_vec_require_capacity(idwc_v0.length + 1);
    idwc_v0.data[idwc_v0.length++] = idwc_t3;
    const int32_t idwc_t4 = INT32_C(20);
    const size_t idwc_t5 = ((size_t)UINT64_C(1));
    idwc_v0.data[idwc_vec_index(idwc_v0.length, idwc_t5)] = idwc_t4;
    const size_t idwc_t6 = idwc_v0.length;
    const size_t idwc_t7 = ((size_t)UINT64_C(1));
    const int32_t idwc_t8 = idwc_v0.data[idwc_vec_index(idwc_v0.length, idwc_t7)];
    const size_t idwc_t9 = ((size_t)UINT64_C(3));
    const int32_t idwc_t10 = idwc_v0.data[idwc_vec_index(idwc_v0.length, idwc_t9)];
    printf("%zu", idwc_t6);
    fputs(" ", stdout);
    printf("%" PRId32, idwc_t8);
    fputs(" ", stdout);
    printf("%" PRId32, idwc_t10);
    putchar('\n');
    fputs("values = ", stdout);
    putchar('[');
    for (size_t idwc_t11 = 0; idwc_t11 < idwc_v0.length; ++idwc_t11) {
        if (idwc_t11 != 0) { fputs(", ", stdout); }
        printf("%" PRId32, idwc_v0.data[idwc_t11]);
    }
    putchar(']');
    putchar('\n');
    idwc_vec_bool idwc_v1 = {{0}, 0};
    idwc_vec_require_capacity(3);
    const bool idwc_t12 = true;
    for (size_t idwc_t13 = 0; idwc_t13 < 3; ++idwc_t13) {
        idwc_v1.data[idwc_v1.length++] = idwc_t12;
    }
    (void)idwc_v1;
    const size_t idwc_t14 = idwc_v1.length;
    const size_t idwc_t15 = ((size_t)UINT64_C(2));
    const bool idwc_t16 = idwc_v1.data[idwc_vec_index(idwc_v1.length, idwc_t15)];
    printf("%zu", idwc_t14);
    fputs(" ", stdout);
    fputs(idwc_t16 ? "true" : "false", stdout);
    putchar('\n');
    fputs("repeated = ", stdout);
    putchar('[');
    for (size_t idwc_t17 = 0; idwc_t17 < idwc_v1.length; ++idwc_t17) {
        if (idwc_t17 != 0) { fputs(", ", stdout); }
        fputs(idwc_v1.data[idwc_t17] ? "true" : "false", stdout);
    }
    putchar(']');
    putchar('\n');
    idwc_vec_i32 idwc_v2 = idwc_v0;
    (void)idwc_v2;
    const size_t idwc_t18 = ((size_t)UINT64_C(0));
    const int32_t idwc_t19 = idwc_v2.data[idwc_vec_index(idwc_v2.length, idwc_t18)];
    printf("%" PRId32, idwc_t19);
    putchar('\n');
    idwc_vec_f64 idwc_v3 = {{0}, 0};
    idwc_vec_require_capacity(1);
    const volatile double idwc_t20 = 0x18000000000000p-52;
    idwc_v3.data[idwc_v3.length++] = idwc_t20;
    (void)idwc_v3;
    const size_t idwc_t21 = ((size_t)UINT64_C(0));
    const double idwc_t22 = idwc_v3.data[idwc_vec_index(idwc_v3.length, idwc_t21)];
    idwc_print_f64(idwc_t22, -1);
    putchar('\n');
    return 0;
}
