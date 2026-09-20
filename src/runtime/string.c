#define IDWC_LINE_LIMIT ((size_t)IDWC_STRING_CAPACITY_PLACEHOLDER)

typedef struct {
    const unsigned char *data;
    size_t length;
} idwc_str;

typedef struct {
    unsigned char data[IDWC_STRING_CAPACITY_PLACEHOLDER];
    size_t length;
} idwc_string;

idwc_string idwc_string_from(idwc_str value) {
    if (value.length > IDWC_LINE_LIMIT) {
        idwc_fail("idwc: String capacity exceeded\n");
    }
    idwc_string result = {{0}, value.length};
    if (value.length != 0) {
        memcpy(result.data, value.data, value.length);
    }
    return result;
}

void idwc_print_str(idwc_str value) {
    if (value.length != 0) {
        (void)fwrite(value.data, 1, value.length, stdout);
    }
}

void idwc_print_string(const idwc_string *value) {
    if (value->length != 0) {
        (void)fwrite(value->data, 1, value->length, stdout);
    }
}
