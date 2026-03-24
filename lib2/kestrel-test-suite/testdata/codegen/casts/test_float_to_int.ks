// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: lang.f64 = 42.7;
    let result = lang.cast_f64_i64(x);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
