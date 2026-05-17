// test: diagnostics
// stdlib: false

module Test
func f32ToF64(f: lang.f32) -> lang.f64 {
    lang.cast_f32_f64(f)
}
func f64ToF32(d: lang.f64) -> lang.f32 {
    lang.cast_f64_f32(d)
}
