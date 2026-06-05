// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let x: lang.i8 = 42;
    let y = lang.cast_i8_i32(x);
    let result = lang.cast_i32_i64(y);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
