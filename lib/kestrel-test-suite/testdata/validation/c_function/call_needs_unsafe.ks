// test: diagnostics
// stdlib: true

// Calling a C function value is unsafe (E641): nothing checks that its
// address is a live C function of that type. Calling an `@export(.C)`
// function by name is an ordinary call.

module Test

type IntFn = @extern(.C) (Int64) -> Int64

struct Hook: FFISafe { var callback: IntFn }

@export(.C)
func exported(x: Int64) -> Int64 { x }

func uses() -> Int64 {
    let f: IntFn = exported;
    let hook = Hook(callback: exported);
    let a = f(1); // ERROR: calling a C function value is unsafe and requires an `unsafe` expression or block
    let b = hook.callback(2); // ERROR: calling a C function value is unsafe and requires an `unsafe` expression or block
    let c = unsafe f(3);
    let d = unsafe hook.callback(4);
    a + b + c + d + exported(5)
}
