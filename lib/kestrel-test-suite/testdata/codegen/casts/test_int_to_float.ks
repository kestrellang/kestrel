// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let x: lang.i64 = 42;
    let f = lang.cast_i64_f64(x);
    // Convert back to check
    let result = lang.cast_f64_i64(f);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
