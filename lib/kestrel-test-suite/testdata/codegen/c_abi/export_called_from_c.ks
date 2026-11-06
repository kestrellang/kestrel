// test: execution
// stdlib: true
// backends: cranelift,llvm
// c-source: c_abi_exports.c
// expect-stdout: 11111\n

// `@export(.C)` functions are called by C under their symbol names with C's
// struct-passing rules: a struct in one register, a float struct (an HFA on
// AArch64), a struct returned through a hidden pointer, SSE + INTEGER, and
// small integers. `abi_call_exports` scores one digit per correct result.

module Test

struct I32x2: FFISafe { var a: Int32; var b: Int32 }
struct F32x3: FFISafe { var x: Float32; var y: Float32; var z: Float32 }
struct Big: FFISafe { var a: Int64; var b: Int64; var c: Int64 }
struct DI: FFISafe { var d: Float64; var i: Int64 }

@export(.C, mangleName: "kestrel_i32x2_swap")
func swap(v: I32x2) -> I32x2 { I32x2(a: v.b, b: v.a) }

@export(.C, mangleName: "kestrel_f32x3_scale")
func scale(v: F32x3, k: Float32) -> F32x3 { F32x3(x: v.x * k, y: v.y * k, z: v.z * k) }

@export(.C, mangleName: "kestrel_big_rotate")
func rotate(v: Big) -> Big { Big(a: v.b, b: v.c, c: v.a) }

@export(.C, mangleName: "kestrel_di_add")
func addDI(v: DI, n: Int64) -> DI { DI(d: v.d, i: v.i + n) }

@export(.C, mangleName: "kestrel_add_u8")
func addU8(a: UInt8, b: Int8) -> Int64 { Int64(from: a) + Int64(from: b) }

@extern(.C, mangleName: "abi_call_exports") func callExports() -> Int64

@main
func main() -> lang.i64 {
    println(unsafe callExports());
    0
}
