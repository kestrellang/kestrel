// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: lang.f64 = 42.0;
    let y = lang.cast_f64_f32(x);
    let z = lang.cast_f32_f64(y);
    let result = lang.cast_f64_i64(z);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
