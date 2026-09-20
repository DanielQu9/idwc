#include <stdio.h>
#include <stdbool.h>
#include <stdint.h>
#include <inttypes.h>
#include <stdlib.h>
#include <signal.h>
#include <string.h>

static void idwc_fail(const char *message) {
#ifdef SIGPIPE
    signal(SIGPIPE, SIG_IGN);
#endif
    fflush(stdout);
    fputs(message, stderr);
    exit(101);
}

#define IDWC_LINE_LIMIT ((size_t)4096)

typedef struct {
    const unsigned char *data;
    size_t length;
} idwc_str;

typedef struct {
    unsigned char data[4096];
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
int main(void) {
    const idwc_str idwc_v0 = ((idwc_str){(const unsigned char *)"\344\275\240\345\245\275", ((size_t)UINT64_C(6))});
    (void)idwc_v0;
    const idwc_str idwc_v1 = ((idwc_str){(const unsigned char *)"!", ((size_t)UINT64_C(1))});
    (void)idwc_v1;
    const idwc_string idwc_t0 = idwc_string_from(idwc_v0);
    idwc_string idwc_v2 = idwc_t0;
    (void)idwc_v2;
    const idwc_string idwc_t1 = idwc_v2;
    const idwc_str idwc_t2 = idwc_v1;
    idwc_print_string(&idwc_t1);
    idwc_print_str(idwc_t2);
    putchar('\n');
    const idwc_string idwc_t3 = idwc_string_from(((idwc_str){(const unsigned char *)"IdwC", ((size_t)UINT64_C(4))}));
    idwc_v2 = idwc_t3;
    const idwc_string idwc_v3 = idwc_v2;
    (void)idwc_v3;
    const idwc_string idwc_t4 = idwc_v3;
    idwc_print_string(&idwc_t4);
    putchar('\n');
    return 0;
}
