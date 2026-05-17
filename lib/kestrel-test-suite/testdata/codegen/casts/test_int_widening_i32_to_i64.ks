// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: lang.i32 = 77;
    let result = lang.cast_i32_i64(x);
    if lang.i64_eq(result, 77) { 0 } else { 1 }
}
