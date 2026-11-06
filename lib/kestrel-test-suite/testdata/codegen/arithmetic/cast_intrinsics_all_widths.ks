// test: execution
// stdlib: false
// backends: cranelift,llvm
// expect-exit: 0

// Every seeded `lang.cast_<from>_<to>` intrinsic has a lowering (the
// `every_seeded_intrinsic_has_a_lowering` unit test). This exercises the
// widths the original table lacked: the source's signedness picks sign- vs
// zero-extension, narrowing truncates, and float->int saturates at the TARGET
// width on both backends (Cranelift converts at 32 bits, then clamps).

module Test
@main
func main() -> lang.i64 {
    let neg3: lang.i64 = lang.i64_sub(0, 3);
    let a: lang.i8 = lang.cast_i64_i8(neg3);
    // sign-extends: i8 -3 -> u32 0xFFFFFFFD == i32 -3
    if lang.i1_not(lang.i32_eq(lang.cast_i8_u32(a), lang.cast_i64_i32(neg3))) { return 1; }
    // zero-extends: bits of 200 in an i8 -> u64 200
    let b: lang.i8 = lang.cast_i64_i8(200);
    if lang.i1_not(lang.i64_eq(lang.cast_u8_u64(b), 200)) { return 2; }
    // truncates: 70000 -> u16 4464
    if lang.i1_not(lang.i16_eq(lang.cast_u64_u16(70000), lang.cast_i64_i16(4464))) { return 3; }
    // i8 -> f64 -> i8 round trip
    let f: lang.f64 = lang.cast_i8_f64(a);
    if lang.i1_not(lang.i8_eq(lang.cast_f64_i8(f), a)) { return 4; }
    // i16 -> f32 -> i16 round trip
    let m12: lang.i16 = lang.cast_i64_i16(lang.i64_sub(0, 12));
    let g: lang.f32 = lang.cast_i16_f32(m12);
    if lang.i1_not(lang.i16_eq(lang.cast_f32_i16(g), m12)) { return 5; }
    // 127 + -32768
    if lang.i1_not(lang.i64_eq(sat(), lang.i64_sub(127, 32768))) { return 6; }
    0
}
func sat() -> lang.i64 {
    // saturation at the target width: 200.0 -> i8 127, -1e6 -> i16 -32768
    let s8: lang.i8 = lang.cast_f64_i8(200.0);
    let s16: lang.i16 = lang.cast_f64_i16(lang.f64_sub(0.0, 1000000.0));
    lang.i64_add(lang.cast_i8_i64(s8), lang.cast_i16_i64(s16))
}
