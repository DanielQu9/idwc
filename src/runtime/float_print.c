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
