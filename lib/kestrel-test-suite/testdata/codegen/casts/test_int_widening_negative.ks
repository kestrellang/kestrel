// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let x: lang.i8 = lang.i8_neg(10);
    let y = lang.cast_i8_i64(x);
    // -10 as i64 should still be -10
    let neg10: lang.i64 = lang.i64_neg(10);
    if lang.i64_eq(y, neg10) { 0 } else { 1 }
}
