// test: diagnostics
// stdlib: false

module Test
func unsignedOps() {
    let _d = lang.i64_unsigned_div(10, 3);
    let _r = lang.i64_unsigned_rem(10, 3);
    let _lt = lang.i64_unsigned_lt(1, 2);
    let _gt = lang.i64_unsigned_gt(2, 1);
    let _le = lang.i64_unsigned_le(1, 1);
    let _ge = lang.i64_unsigned_ge(1, 1);
    let _shr = lang.i64_unsigned_shr(8, 2);
}
