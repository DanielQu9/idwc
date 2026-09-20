#define IDWC_LINE_LIMIT ((size_t)4096)
#define IDWC_TOKEN_LIMIT ((size_t)256)

typedef struct {
    unsigned char data[4096];
    size_t length;
} idwc_string;

typedef struct {
    const idwc_string *source;
    size_t length;
    size_t start[256];
    size_t span[256];
} idwc_tokens;

uint32_t idwc_utf8_next(const unsigned char *data, size_t length, size_t *index) {
    if (*index >= length) { idwc_fail("idwc: invalid UTF-8 input\n"); }
    unsigned first = data[(*index)++];
    if (first < 0x80) { return first; }
    unsigned count = 0;
    uint32_t value = 0;
    uint32_t minimum = 0;
    if (first >= 0xc2 && first <= 0xdf) {
        count = 1; value = first & 0x1f; minimum = 0x80;
    } else if (first >= 0xe0 && first <= 0xef) {
        count = 2; value = first & 0x0f; minimum = 0x800;
    } else if (first >= 0xf0 && first <= 0xf4) {
        count = 3; value = first & 0x07; minimum = 0x10000;
    } else {
        idwc_fail("idwc: invalid UTF-8 input\n");
    }
    for (unsigned i = 0; i < count; ++i) {
        if (*index >= length || (data[*index] & 0xc0) != 0x80) {
            idwc_fail("idwc: invalid UTF-8 input\n");
        }
        value = (value << 6) | (data[(*index)++] & 0x3f);
    }
    if (value < minimum || value > 0x10ffff || (value >= 0xd800 && value <= 0xdfff)) {
        idwc_fail("idwc: invalid UTF-8 input\n");
    }
    return value;
}

bool idwc_unicode_space(uint32_t value) {
    return (value >= 0x0009 && value <= 0x000d) || value == 0x0020
        || value == 0x0085 || value == 0x00a0 || value == 0x1680
        || (value >= 0x2000 && value <= 0x200a) || value == 0x2028
        || value == 0x2029 || value == 0x202f || value == 0x205f
        || value == 0x3000;
}

void idwc_validate_utf8(const idwc_string *value) {
    size_t index = 0;
    while (index < value->length) { (void)idwc_utf8_next(value->data, value->length, &index); }
}

void idwc_read_line(idwc_string *value) {
    for (;;) {
        int byte = fgetc(stdin);
        if (byte == EOF) {
            if (ferror(stdin)) { idwc_fail("idwc: stdin I/O error\n"); }
            break;
        }
        if (value->length == IDWC_LINE_LIMIT) {
            idwc_fail("idwc: input line buffer too long\n");
        }
        value->data[value->length++] = (unsigned char)byte;
        if (byte == '\n') { break; }
    }
    idwc_validate_utf8(value);
}

idwc_tokens idwc_split_whitespace(const idwc_string *source) {
    idwc_tokens result = {source, 0, {0}, {0}};
    size_t index = 0;
    while (index < source->length) {
        size_t position = index;
        uint32_t value = idwc_utf8_next(source->data, source->length, &index);
        if (idwc_unicode_space(value)) { continue; }
        size_t start = position;
        size_t end = index;
        while (index < source->length) {
            position = index;
            value = idwc_utf8_next(source->data, source->length, &index);
            if (idwc_unicode_space(value)) { break; }
            end = index;
        }
        if (result.length == IDWC_TOKEN_LIMIT) {
            idwc_fail("idwc: too many input tokens\n");
        }
        result.start[result.length] = start;
        result.span[result.length] = end - start;
        ++result.length;
    }
    return result;
}

void idwc_trim_span(const idwc_string *source, const unsigned char **data, size_t *length) {
    size_t index = 0;
    size_t start = source->length;
    size_t end = 0;
    while (index < source->length) {
        size_t position = index;
        uint32_t value = idwc_utf8_next(source->data, source->length, &index);
        if (!idwc_unicode_space(value)) {
            if (start == source->length) { start = position; }
            end = index;
        }
    }
    if (start == source->length) { start = 0; }
    *data = source->data + start;
    *length = end - start;
}

void idwc_token_span(
    const idwc_tokens *tokens,
    size_t index,
    const unsigned char **data,
    size_t *length
) {
    if (index >= tokens->length) { idwc_fail("idwc: token index out of bounds\n"); }
    *data = tokens->source->data + tokens->start[index];
    *length = tokens->span[index];
}
