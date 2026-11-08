module Main

import std.system.Process
import std.memory.RawPointer
import std.ffi.malloc
import std.streams.StandardOutput

// GMP through its C ABI, the same library the C, Rust and Go programs use.
// An mpz_t is a 16-byte struct; each one lives in its own malloc'd block.
@extern(.C, mangleName: "__gmpz_init")
func mpzInit(z: RawPointer)
@extern(.C, mangleName: "__gmpz_init_set_ui")
func mpzInitSetUi(z: RawPointer, v: UInt64)
@extern(.C, mangleName: "__gmpz_mul_ui")
func mpzMulUi(r: RawPointer, a: RawPointer, v: UInt64)
@extern(.C, mangleName: "__gmpz_add")
func mpzAdd(r: RawPointer, a: RawPointer, b: RawPointer)
@extern(.C, mangleName: "__gmpz_addmul_ui")
func mpzAddmulUi(r: RawPointer, a: RawPointer, v: UInt64)
@extern(.C, mangleName: "__gmpz_submul_ui")
func mpzSubmulUi(r: RawPointer, a: RawPointer, v: UInt64)
@extern(.C, mangleName: "__gmpz_tdiv_q")
func mpzTdivQ(q: RawPointer, n: RawPointer, d: RawPointer)
@extern(.C, mangleName: "__gmpz_get_ui")
func mpzGetUi(z: RawPointer) -> UInt64
@extern(.C, mangleName: "__gmpz_cmp")
func mpzCmp(a: RawPointer, b: RawPointer) -> Int32

func newMpz(v: UInt64) -> RawPointer {
    let z = unsafe malloc(16);
    unsafe mpzInitSetUi(z, v);
    z
}

@main
func main() {
    let args = Process.arguments();
    let n = if args.count > 1 { Int64(parsing: args(1)) ?? 27 } else { 27 };
    let tmp1 = newMpz(0);
    let tmp2 = newMpz(0);
    let acc = newMpz(0);
    let den = newMpz(1);
    let num = newMpz(1);

    var out = StandardOutput();
    var line = Array[UInt8](repeating: 0, count: 32);
    var col: Int64 = 0;
    var i: Int64 = 0;
    var k: UInt64 = 0;
    while i < n {
        k = k + 1;
        // next_term(k)
        let k2 = k * 2 + 1;
        unsafe mpzAddmulUi(acc, num, 2);
        unsafe mpzMulUi(acc, acc, k2);
        unsafe mpzMulUi(den, den, k2);
        unsafe mpzMulUi(num, num, k);
        if unsafe mpzCmp(num, acc) > 0 { continue }
        // extract_digit(3) and extract_digit(4)
        unsafe mpzMulUi(tmp1, num, 3);
        unsafe mpzAdd(tmp2, tmp1, acc);
        unsafe mpzTdivQ(tmp1, tmp2, den);
        let d = unsafe mpzGetUi(tmp1);
        unsafe mpzMulUi(tmp1, num, 4);
        unsafe mpzAdd(tmp2, tmp1, acc);
        unsafe mpzTdivQ(tmp1, tmp2, den);
        if d != unsafe mpzGetUi(tmp1) { continue }

        line(col) = UInt8(truncating: d) + 48;
        col = col + 1;
        i = i + 1;
        if i % 10 == 0 {
            let _ = out.write(from: line(0..<col));
            let _ = out.write(text: "\t:\(i)\n");
            col = 0;
        }
        // eliminate_digit(d)
        unsafe mpzSubmulUi(acc, den, d);
        unsafe mpzMulUi(acc, acc, 10);
        unsafe mpzMulUi(num, num, 10);
    }
    if col > 0 {
        let _ = out.write(from: line(0..<col));
        for _ in col..<10 { let _ = out.write(text: " "); }
        let _ = out.write(text: "\t:\(i)\n");
    }
    let _ = out.flush();
}
