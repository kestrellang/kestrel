// test: diagnostics
// stdlib: false
//
// Every `@extern` function is unsafe to call: the compiler can't check
// foreign code, and there is no opt-out.

module Test

@extern(.C, mangleName: "abs")
func c_abs(x: lang.i32) -> lang.i32

func f() -> lang.i32 {
    c_abs(1) // ERROR: extern function 'c_abs' is unsafe and requires an `unsafe` expression or block
}

func g() -> lang.i32 {
    unsafe c_abs(1)
}

func h() -> (lang.i32) -> lang.i32 {
    c_abs // ERROR: extern function 'c_abs' is unsafe and requires an `unsafe` expression or block
}
