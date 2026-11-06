// test: execution
// stdlib: true
// backends: cranelift,llvm
// c-source: c_abi_helpers.c
// expect-stdout: 12\n34\n506\n78\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\n123\n456\n1020304\n15010\n

// Structs passed to and returned from C by value, one per x86-64 / AArch64
// classification: a single integer register, two integer registers, float
// registers (an HFA on AArch64), a float pair packed into one SSE register,
// a mixed int/float eightbyte, SSE + INTEGER, and a struct too big for
// registers. Before the C ABI classifier these all went as a pointer.

module Test

struct I32x2: FFISafe { var a: Int32; var b: Int32 }
struct I64x2: FFISafe { var a: Int64; var b: Int64 }
struct F64x2: FFISafe { var x: Float64; var y: Float64 }
struct F32x2: FFISafe { var x: Float32; var y: Float32 }
struct F32x3: FFISafe { var x: Float32; var y: Float32; var z: Float32 }
struct Mixed: FFISafe { var i: Int32; var f: Float32 }
struct DI: FFISafe { var d: Float64; var i: Int64 }
struct Big: FFISafe { var a: Int64; var b: Int64; var c: Int64 }
struct Rgba: FFISafe { var r: UInt8; var g: UInt8; var b: UInt8; var a: UInt8 }

@extern(.C, mangleName: "abi_i32x2_sum") func i32x2Sum(v: I32x2) -> Int64
@extern(.C, mangleName: "abi_i32x2_make") func i32x2Make(a: Int32, b: Int32) -> I32x2
@extern(.C, mangleName: "abi_i64x2_sum") func i64x2Sum(v: I64x2) -> Int64
@extern(.C, mangleName: "abi_i64x2_make") func i64x2Make(a: Int64, b: Int64) -> I64x2
@extern(.C, mangleName: "abi_f64x2_sum") func f64x2Sum(v: F64x2) -> Float64
@extern(.C, mangleName: "abi_f64x2_make") func f64x2Make(x: Float64, y: Float64) -> F64x2
@extern(.C, mangleName: "abi_f32x2_sum") func f32x2Sum(v: F32x2) -> Float32
@extern(.C, mangleName: "abi_f32x2_make") func f32x2Make(x: Float32, y: Float32) -> F32x2
@extern(.C, mangleName: "abi_f32x3_sum") func f32x3Sum(v: F32x3) -> Float32
@extern(.C, mangleName: "abi_f32x3_make") func f32x3Make(x: Float32, y: Float32, z: Float32) -> F32x3
@extern(.C, mangleName: "abi_mixed_sum") func mixedSum(v: Mixed) -> Float64
@extern(.C, mangleName: "abi_mixed_make") func mixedMake(i: Int32, f: Float32) -> Mixed
@extern(.C, mangleName: "abi_di_sum") func diSum(v: DI) -> Float64
@extern(.C, mangleName: "abi_di_make") func diMake(d: Float64, i: Int64) -> DI
@extern(.C, mangleName: "abi_big_sum") func bigSum(v: Big) -> Int64
@extern(.C, mangleName: "abi_big_make") func bigMake(a: Int64, b: Int64, c: Int64) -> Big
@extern(.C, mangleName: "abi_rgba_sum") func rgbaSum(v: Rgba) -> Int64
@extern(.C, mangleName: "abi_after_six") func afterSix(a: Int64, b: Int64, c: Int64, d: Int64, e: Int64, v: I64x2) -> Int64

@main
func main() -> lang.i64 {
    unsafe {
        println(i32x2Sum(I32x2(a: 1, b: 2)));
        let p = i32x2Make(3, 4);
        println(Int64(from: p.a) * 10 + Int64(from: p.b));
        println(i64x2Sum(I64x2(a: 5, b: 6)));
        let q = i64x2Make(7, 8);
        println(q.a * 10 + q.b);
        println(f64x2Sum(F64x2(x: 1.5, y: 2.0)) == 17.0);
        let d = f64x2Make(0.5, 0.25);
        println(d.x == 0.5 and d.y == 0.25);
        let fx: Float32 = 1.5;
        let fy: Float32 = 2.0;
        println(f32x2Sum(F32x2(x: fx, y: fy)) == 17.0);
        let f2 = f32x2Make(fx, fy);
        println(f2.x == 1.5 and f2.y == 2.0);
        let fz: Float32 = 3.0;
        println(f32x3Sum(F32x3(x: fx, y: fy, z: fz)) == 173.0);
        let f3 = f32x3Make(fx, fy, fz);
        println(f3.x == 1.5 and f3.y == 2.0 and f3.z == 3.0);
        let half: Float32 = 0.5;
        println(mixedSum(Mixed(i: 4, f: half)) == 40.5);
        let m = mixedMake(9, half);
        println(m.i == 9 and m.f == 0.5);
        println(diSum(DI(d: 2.5, i: 3)) == 28.0);
        let di = diMake(0.75, 11);
        println(di.d == 0.75 and di.i == 11);
        println(bigSum(Big(a: 1, b: 2, c: 3)));
        let b = bigMake(4, 5, 6);
        println(b.a * 100 + b.b * 10 + b.c);
        println(rgbaSum(Rgba(r: 1, g: 2, b: 3, a: 4)));
        println(afterSix(1, 2, 3, 4, 5, I64x2(a: 15, b: -5)));
    }
    0
}
