/* Only binary64 with one rounded result per operation is accepted. */
#if defined(__FAST_MATH__) || (defined(__FINITE_MATH_ONLY__) && __FINITE_MATH_ONLY__ > 0)
#error "idwc f64 requires fast-math to be disabled"
#endif
_Static_assert(FLT_RADIX == 2 && DBL_MANT_DIG == 53 && DBL_MAX_EXP == 1024
    && DBL_MIN_EXP == -1021 && sizeof(double) == sizeof(uint64_t),
    "idwc f64 requires IEEE binary64 double");
_Static_assert(FLT_EVAL_METHOD == 0, "idwc f64 rejects excess floating-point precision");

/* Establish a nontrapping, nearest-even environment and the C decimal locale. */
static void idwc_float_init(void) {
    double one = 1.0;
    uint64_t bits;
    memcpy(&bits, &one, sizeof bits);
    if (bits != UINT64_C(0x3ff0000000000000)
        || setlocale(LC_NUMERIC, "C") == NULL
        || fesetenv(FE_DFL_ENV) != 0 || fesetround(FE_TONEAREST) != 0) {
        idwc_fail("idwc: unsupported floating-point environment\n");
    }
    volatile double minimum = DBL_MIN;
    volatile double tiny = DBL_TRUE_MIN;
    if (minimum / 2.0 == 0.0 || tiny * 1.0 == 0.0) {
        idwc_fail("idwc: unsupported floating-point environment\n");
    }
}

