// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: lang.i8 = 42;
    let result = lang.cast_i8_i64(x);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
