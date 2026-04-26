// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    // 256 truncated to i8 should be 0
    let x: lang.i64 = 256;
    let y = lang.cast_i64_i8(x);
    let result = lang.cast_i8_i64(y);
    if lang.i64_eq(result, 0) { 0 } else { 1 }
}
