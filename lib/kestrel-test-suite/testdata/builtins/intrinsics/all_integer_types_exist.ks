// test: diagnostics
// stdlib: false

module Test
func allInts() {
    let _i1: lang.i1 = true;
    let _i8: lang.i8 = lang.cast_i64_i8(0);
    let _i16: lang.i16 = lang.cast_i64_i16(0);
    let _i32: lang.i32 = lang.cast_i64_i32(0);
    let _i64: lang.i64 = 0;
}
