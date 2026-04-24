// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: lang.i64 = 42;
    let y = lang.cast_i64_i32(x);
    let result = lang.cast_i32_i64(y);
    if lang.i64_eq(result, 42) { 0 } else { 1 }
}
