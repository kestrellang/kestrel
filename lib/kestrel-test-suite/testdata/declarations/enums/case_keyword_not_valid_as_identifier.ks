// test: diagnostics
// stdlib: false

module Test

func case() -> lang.i64 { // ERROR
    42
}
