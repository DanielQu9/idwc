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

