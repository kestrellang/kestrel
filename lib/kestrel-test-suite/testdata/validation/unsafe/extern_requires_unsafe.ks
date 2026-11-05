// test: diagnostics
// stdlib: false
//
// An `@extern` function is unsafe to call unless it is declared
// `safe: true`, in which case the declaration vouches for the C contract.

module Test

@extern(.C, mangleName: "abs")
func c_abs(x: lang.i32) -> lang.i32

@extern(.C, mangleName: "labs", safe: true)
func c_labs(x: lang.i64) -> lang.i64

func f() -> lang.i32 {
    c_abs(1) // ERROR: extern function 'c_abs' is unsafe and requires an `unsafe` expression or block
}

func g() -> lang.i32 {
    unsafe c_abs(1)
}

func h() -> lang.i64 {
    c_labs(1)
}
