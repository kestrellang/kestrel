// test: diagnostics
// stdlib: false

module Test

func main() {
    let x: lang.i64 = 42;
    let y = x[lang.i64]; // ERROR: type
}
