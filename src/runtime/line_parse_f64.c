double idwc_parse_f64_span(const unsigned char *data, size_t length) {
    if (length == 3 && memcmp(data, "NaN", 3) == 0) { return NAN; }
    if (length == 3 && memcmp(data, "inf", 3) == 0) { return INFINITY; }
    if (length == 4 && memcmp(data, "+inf", 4) == 0) { return INFINITY; }
    if (length == 4 && memcmp(data, "-inf", 4) == 0) { return -INFINITY; }
    size_t index = length > 0 && (data[0] == '+' || data[0] == '-') ? 1 : 0;
    size_t digits = 0;
    while (index < length && data[index] >= '0' && data[index] <= '9') {
        ++digits; ++index;
    }
    if (index < length && data[index] == '.') {
        ++index;
        while (index < length && data[index] >= '0' && data[index] <= '9') {
            ++digits; ++index;
        }
    }
    if (digits == 0) { idwc_fail("idwc: invalid float\n"); }
    if (index < length && (data[index] == 'e' || data[index] == 'E')) {
        ++index;
        if (index < length && (data[index] == '+' || data[index] == '-')) { ++index; }
        size_t start = index;
        while (index < length && data[index] >= '0' && data[index] <= '9') { ++index; }
        if (start == index) { idwc_fail("idwc: invalid float\n"); }
    }
    if (index != length) { idwc_fail("idwc: invalid float\n"); }
    char token[4097];
    memcpy(token, data, length);
    token[length] = '\0';
    char *end;
    double value = strtod(token, &end);
    if (end != token + length) { idwc_fail("idwc: invalid float\n"); }
    return value;
}

double idwc_parse_trimmed_f64(const idwc_string *source) {
    const unsigned char *data;
    size_t length;
    idwc_trim_span(source, &data, &length);
    return idwc_parse_f64_span(data, length);
}

double idwc_parse_token_f64(const idwc_tokens *tokens, size_t index) {
    const unsigned char *data;
    size_t length;
    idwc_token_span(tokens, index, &data, &length);
    return idwc_parse_f64_span(data, length);
}

