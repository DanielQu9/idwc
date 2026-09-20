#define IDWC_VEC_LIMIT ((size_t)IDWC_VEC_CAPACITY_PLACEHOLDER)

typedef struct { int32_t data[IDWC_VEC_CAPACITY_PLACEHOLDER]; size_t length; } idwc_vec_i32;
typedef struct { size_t data[IDWC_VEC_CAPACITY_PLACEHOLDER]; size_t length; } idwc_vec_usize;
typedef struct { double data[IDWC_VEC_CAPACITY_PLACEHOLDER]; size_t length; } idwc_vec_f64;
typedef struct { bool data[IDWC_VEC_CAPACITY_PLACEHOLDER]; size_t length; } idwc_vec_bool;

void idwc_vec_require_capacity(size_t requested) {
    if (requested > IDWC_VEC_LIMIT) {
        idwc_fail("idwc: Vec capacity exceeded\n");
    }
}

size_t idwc_vec_index(size_t length, size_t index) {
    if (index >= length) {
        idwc_fail("idwc: Vec index out of bounds\n");
    }
    return index;
}
