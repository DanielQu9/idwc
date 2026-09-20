#include <stdio.h>
#include <stdbool.h>
#include <stdint.h>
#include <inttypes.h>
#include <stdlib.h>
#include <signal.h>

static void idwc_fail(const char *message) {
#ifdef SIGPIPE
    signal(SIGPIPE, SIG_IGN);
#endif
    fflush(stdout);
    fputs(message, stderr);
    exit(101);
}

static int32_t idwc_checked(int64_t value) {
    if (value < INT32_MIN || value > INT32_MAX) {
        idwc_fail("idwc: integer overflow\n");
    }
    return (int32_t)value;
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

static int32_t idwc_add(int32_t a, int32_t b) {
    return idwc_checked((int64_t)a + (int64_t)b);
}

static void idwc_flush_stdout(void) {
#ifdef SIGPIPE
    signal(SIGPIPE, SIG_IGN);
#endif
    if (fflush(stdout) == EOF || ferror(stdout)) {
        idwc_fail("idwc: stdout flush error\n");
    }
}

static int32_t idwc_read_i32(void) {
    unsigned char token[129];
    size_t length = idwc_read_token(token);
    bool negative = token[0] == '-';
    size_t start = (negative || token[0] == '+') ? 1 : 0;
    if (start == length) {
        idwc_fail("idwc: invalid integer\n");
    }
    for (size_t i = start; i < length; ++i) {
        if (token[i] < '0' || token[i] > '9') {
            idwc_fail("idwc: invalid integer\n");
        }
    }
    uint64_t limit = negative ? UINT64_C(2147483648) : UINT64_C(2147483647);
    uint64_t value = 0;
    for (size_t i = start; i < length; ++i) {
        uint64_t digit = token[i] - '0';
        if (value > (limit - digit) / 10) {
            idwc_fail("idwc: integer out of range\n");
        }
        value = value * 10 + digit;
    }
    if (negative && value == UINT64_C(2147483648)) {
        return INT32_MIN;
    }
    return negative ? -(int32_t)value : (int32_t)value;
}

int32_t idwc_f1(const int32_t idwc_v0, const int32_t idwc_v1);
bool idwc_f2(const int32_t idwc_v0);

int32_t idwc_f1(const int32_t idwc_v0, const int32_t idwc_v1) {
    (void)idwc_v0;
    (void)idwc_v1;
    const int32_t idwc_t0 = idwc_add(idwc_v0, idwc_v1);
    return idwc_t0;
}

bool idwc_f2(const int32_t idwc_v0) {
    (void)idwc_v0;
    const bool idwc_t1 = (idwc_v0 > INT32_C(0));
    if (idwc_t1) {
        return true;
    }
    return false;
}

int main(void) {
    fputs("Enter two integers: ", stdout);
    idwc_flush_stdout();
    (void)(0);
    const int32_t idwc_t2 = idwc_read_i32();
    const int32_t idwc_v0 = idwc_t2;
    (void)idwc_v0;
    const int32_t idwc_t3 = idwc_read_i32();
    const int32_t idwc_v1 = idwc_t3;
    (void)idwc_v1;
    const int32_t idwc_t4 = idwc_v0;
    const int32_t idwc_t5 = idwc_v1;
    const int32_t idwc_t6 = idwc_f1(idwc_t4, idwc_t5);
    const int32_t idwc_v2 = idwc_t6;
    (void)idwc_v2;
    const int32_t idwc_t7 = idwc_v2;
    const int32_t idwc_t8 = idwc_v2;
    const bool idwc_t9 = idwc_f2(idwc_t8);
    const bool idwc_t10 = idwc_t9;
    fputs("sum = ", stdout);
    printf("%" PRId32, idwc_t7);
    fputs(", positive = ", stdout);
    fputs(idwc_t10 ? "true" : "false", stdout);
    putchar('\n');
    return 0;
}
