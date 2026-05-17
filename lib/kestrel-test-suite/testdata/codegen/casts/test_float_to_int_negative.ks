// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: lang.f64 = lang.f64_neg(5.9);
    let y = lang.cast_f64_i64(x);
    // Should truncate toward zero, so -5
    let neg5: lang.i64 = lang.i64_neg(5);
    if lang.i64_eq(y, neg5) { 0 } else { 1 }
}
