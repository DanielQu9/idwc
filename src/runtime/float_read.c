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

