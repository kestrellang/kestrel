// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    let x: lang.i8 = 20;
    let y: lang.i8 = 22;
    let result = lang.i64_add(lang.cast_i8_i64(x), lang.cast_i8_i64(y));
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
