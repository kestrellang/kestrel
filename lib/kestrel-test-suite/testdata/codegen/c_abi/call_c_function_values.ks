// test: execution
// stdlib: true
// backends: cranelift,llvm
// c-source: c_abi_callbacks.c
// expect-stdout: 42\n7\n-1\n44\n-5\n11\n106\ntrue\n24\nfalse\n

// Kestrel calls C function values with the C ABI: ones C hands back, an
// `@export(.C)` function's own C entry point, one stored in a struct field,
// and one read out of a `NullableCFunction`. Small integers keep C's
// zero/sign extension.

module Test

struct I32x2: FFISafe { var a: Int32; var b: Int32 }

type IntFn = @extern(.C) (Int64, Pointer[Int64]) -> Int64

struct Hook: FFISafe {
    var callback: IntFn;
    var userdata: Pointer[Int64];
}

@export(.C, mangleName: "kestrel_add_userdata_2")
func addUserdata(x: Int64, userdata: Pointer[Int64]) -> Int64 {
    x + unsafe userdata.read()
}

@extern(.C, mangleName: "cb_get_double")
func getDouble() -> IntFn
@extern(.C, mangleName: "cb_get_sum_pair")
func getSumPair() -> @extern(.C) (I32x2) -> I32x2
@extern(.C, mangleName: "cb_get_u8_add")
func getU8Add() -> @extern(.C) (UInt8, UInt8) -> UInt8
@extern(.C, mangleName: "cb_get_i8_neg")
func getI8Neg() -> @extern(.C) (Int8) -> Int8
@extern(.C, mangleName: "cb_maybe_double")
func maybeDouble(which: Int64) -> NullableCFunction[IntFn]

@main
func main() -> lang.i64 {
    var offset: Int64 = 100;
    let state = Pointer(to: offset);
    unsafe {
        let double = getDouble();
        println(double(21, state));
        let sum = getSumPair()(I32x2(a: 3, b: 4));
        println(sum.a);
        println(sum.b);
        println(Int64(from: getU8Add()(200, 100)));
        println(Int64(from: getI8Neg()(5)));
        let own: IntFn = addUserdata;
        offset = 10;
        println(own(1, state));
        let hook = Hook(callback: addUserdata, userdata: state);
        offset = 6;
        println(hook.callback(100, hook.userdata));
        let maybe = maybeDouble(1);
        println(maybe.function.isSome);
        if let .Some(f) = maybe.function {
            println(f(12, state));
        }
        println(maybeDouble(0).function.isSome);
    }
    0
}
