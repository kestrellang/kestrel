// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let x: lang.i64 = 10;
    let f = lang.cast_i64_f64(x);
    let g = lang.f64_add(f, 0.5);
    // 10.5 truncated should be 10
    let result = lang.cast_f64_i64(g);
    if lang.i64_eq(result, 10) { 0 } else { 1 }
}
