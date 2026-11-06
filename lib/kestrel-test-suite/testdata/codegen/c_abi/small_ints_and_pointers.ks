// test: execution
// stdlib: true
// backends: cranelift,llvm
// c-source: c_abi_helpers.c
// expect-stdout: 200\n-5\n60000\n-300\n1\n0\n200\n-5\n506\n608\n

// 8- and 16-bit arguments are widened by the caller (zero- or sign-extended
// by their signedness), and reference params cross as C pointers: `&T` is a
// `const T*`, `&mutating T` a `T*` the C side writes through.

module Test

struct I64x2: FFISafe { var a: Int64; var b: Int64 }

@extern(.C, mangleName: "abi_u8") func u8(x: UInt8) -> Int64
@extern(.C, mangleName: "abi_i8") func i8(x: Int8) -> Int64
@extern(.C, mangleName: "abi_u16") func u16(x: UInt16) -> Int64
@extern(.C, mangleName: "abi_i16") func i16(x: Int16) -> Int64
@extern(.C, mangleName: "abi_bool") func boolean(x: Bool) -> Int64
@extern(.C, mangleName: "abi_ret_u8") func retU8() -> UInt8
@extern(.C, mangleName: "abi_ret_i8") func retI8() -> Int8
@extern(.C, mangleName: "abi_read_ptr") func readPtr(v: &I64x2) -> Int64
@extern(.C, mangleName: "abi_write_ptr") func writePtr(v: &mutating I64x2)

@main
func main() -> lang.i64 {
    unsafe {
        println(u8(200));
        println(i8(-5));
        println(u16(60000));
        println(i16(-300));
        println(boolean(true));
        println(boolean(false));
        println(Int64(from: retU8()));
        println(Int64(from: retI8()));
        var v = I64x2(a: 5, b: 6);
        println(readPtr(v));
        writePtr(v);
        println(v.a * 100 + v.b);
    }
    0
}
