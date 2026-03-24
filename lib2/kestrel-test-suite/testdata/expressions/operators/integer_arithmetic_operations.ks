// test: diagnostics
// stdlib: false

module Main

func sum() -> lang.i64 {
    lang.i64_add(1, 2)
}

func diff() -> lang.i64 {
    lang.i64_sub(5, 3)
}

func product() -> lang.i64 {
    lang.i64_mul(4, 5)
}

func quotient() -> lang.i64 {
    lang.i64_signed_div(10, 2)
}

func remainder() -> lang.i64 {
    lang.i64_signed_rem(10, 3)
}
