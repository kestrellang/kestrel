// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let x: lang.f32 = 42.0;
    let y = lang.cast_f32_f64(x);
    let result = lang.cast_f64_i64(y);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
