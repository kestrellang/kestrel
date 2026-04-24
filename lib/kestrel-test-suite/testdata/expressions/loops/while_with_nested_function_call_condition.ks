// test: diagnostics
// stdlib: false

module Main

func getValue() -> lang.i64 {
    5
}

func isValid(x: lang.i64) -> lang.i1 {
    lang.i64_signed_gt(x, 0)
}

func test() {
    while isValid(getValue()) {
        break;
    }
}
