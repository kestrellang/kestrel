// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    let x: lang.i16 = 100;
    let result = lang.cast_i16_i64(x);
    if lang.i64_eq(result, 100) { 0 } else { 1 }
}
