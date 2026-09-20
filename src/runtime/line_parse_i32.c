int32_t idwc_parse_i32_span(const unsigned char *data, size_t length) {
    bool negative = length > 0 && data[0] == '-';
    size_t start = length > 0 && (negative || data[0] == '+') ? 1 : 0;
    if (start == length) { idwc_fail("idwc: invalid integer\n"); }
    for (size_t i = start; i < length; ++i) {
        if (data[i] < '0' || data[i] > '9') { idwc_fail("idwc: invalid integer\n"); }
    }
    uint64_t limit = negative ? UINT64_C(2147483648) : UINT64_C(2147483647);
    uint64_t value = 0;
    for (size_t i = start; i < length; ++i) {
        uint64_t digit = data[i] - '0';
        if (value > (limit - digit) / 10) { idwc_fail("idwc: integer out of range\n"); }
        value = value * 10 + digit;
    }
    if (negative && value == UINT64_C(2147483648)) { return INT32_MIN; }
    return negative ? -(int32_t)value : (int32_t)value;
}

int32_t idwc_parse_trimmed_i32(const idwc_string *source) {
    const unsigned char *data;
    size_t length;
    idwc_trim_span(source, &data, &length);
    return idwc_parse_i32_span(data, length);
}

int32_t idwc_parse_token_i32(const idwc_tokens *tokens, size_t index) {
    const unsigned char *data;
    size_t length;
    idwc_token_span(tokens, index, &data, &length);
    return idwc_parse_i32_span(data, length);
}

