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

static void idwc_flush_stdout(void) {
#ifdef SIGPIPE
    signal(SIGPIPE, SIG_IGN);
#endif
    if (fflush(stdout) == EOF || ferror(stdout)) {
        idwc_fail("idwc: stdout flush error\n");
    }
}

double idwc_f1(const double idwc_v0, const double idwc_v1);

double idwc_f1(const double idwc_v0, const double idwc_v1) {
    (void)idwc_v0;
    (void)idwc_v1;
    const volatile double idwc_t0 = (idwc_v1 * idwc_v1);
    const volatile double idwc_t1 = idwc_fdiv(idwc_v0, idwc_t0);
    return idwc_t1;
}

int main(void) {
    idwc_float_init();
    fputs("Enter weight (kg) and height (m): ", stdout);
    idwc_flush_stdout();
    (void)(0);
    const volatile double idwc_t2 = idwc_read_f64();
    const double idwc_v0 = idwc_t2;
    (void)idwc_v0;
    const volatile double idwc_t3 = idwc_read_f64();
    const double idwc_v1 = idwc_t3;
    (void)idwc_v1;
    const double idwc_t4 = idwc_v0;
    const double idwc_t5 = idwc_v1;
    const volatile double idwc_t6 = idwc_f1(idwc_t4, idwc_t5);
    const double idwc_v2 = idwc_t6;
    (void)idwc_v2;
    const double idwc_t7 = idwc_v2;
    const bool idwc_t8 = (idwc_v2 < 0x19000000000000p-48);
    const bool idwc_t9 = idwc_t8;
    fputs("BMI = ", stdout);
    idwc_print_f64(idwc_t7, 2);
    fputs(", below 25 = ", stdout);
    fputs(idwc_t9 ? "true" : "false", stdout);
    putchar('\n');
    return 0;
}
