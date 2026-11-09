module Main

import std.system.Process
import std.streams.StandardOutput

// Mandelbrot set as a PBM bitmap. Each output byte is 8 pixels; the 8 lanes
// are written out as separate scalars so LLVM's SLP vectorizer can pack them
// into SIMD registers. Iteration stops early once all 8 lanes have escaped.
func renderByte(crow: Slice[Float64], x: Int64, ci: Float64) -> UInt8 {
        let cr0 = unsafe crow(unchecked: x + 0);
        let cr1 = unsafe crow(unchecked: x + 1);
        let cr2 = unsafe crow(unchecked: x + 2);
        let cr3 = unsafe crow(unchecked: x + 3);
        let cr4 = unsafe crow(unchecked: x + 4);
        let cr5 = unsafe crow(unchecked: x + 5);
        let cr6 = unsafe crow(unchecked: x + 6);
        let cr7 = unsafe crow(unchecked: x + 7);
        var zr0 = cr0; var zi0 = ci;
        var zr1 = cr1; var zi1 = ci;
        var zr2 = cr2; var zi2 = ci;
        var zr3 = cr3; var zi3 = ci;
        var zr4 = cr4; var zi4 = ci;
        var zr5 = cr5; var zi5 = ci;
        var zr6 = cr6; var zi6 = ci;
        var zr7 = cr7; var zi7 = ci;
        var tr0 = zr0 * zr0; var ti0 = zi0 * zi0;
        var tr1 = zr1 * zr1; var ti1 = zi1 * zi1;
        var tr2 = zr2 * zr2; var ti2 = zi2 * zi2;
        var tr3 = zr3 * zr3; var ti3 = zi3 * zi3;
        var tr4 = zr4 * zr4; var ti4 = zi4 * zi4;
        var tr5 = zr5 * zr5; var ti5 = zi5 * zi5;
        var tr6 = zr6 * zr6; var ti6 = zi6 * zi6;
        var tr7 = zr7 * zr7; var ti7 = zi7 * zi7;
    var i: Int64 = 0;
    // 50 iterations in total: 1 above, then 7 rounds of 7, checking escape
    // after every round.
    while i < 7 {
        zi0 = (zr0 + zr0) * zi0 + ci; zr0 = tr0 - ti0 + cr0; tr0 = zr0 * zr0; ti0 = zi0 * zi0;
        zi1 = (zr1 + zr1) * zi1 + ci; zr1 = tr1 - ti1 + cr1; tr1 = zr1 * zr1; ti1 = zi1 * zi1;
        zi2 = (zr2 + zr2) * zi2 + ci; zr2 = tr2 - ti2 + cr2; tr2 = zr2 * zr2; ti2 = zi2 * zi2;
        zi3 = (zr3 + zr3) * zi3 + ci; zr3 = tr3 - ti3 + cr3; tr3 = zr3 * zr3; ti3 = zi3 * zi3;
        zi4 = (zr4 + zr4) * zi4 + ci; zr4 = tr4 - ti4 + cr4; tr4 = zr4 * zr4; ti4 = zi4 * zi4;
        zi5 = (zr5 + zr5) * zi5 + ci; zr5 = tr5 - ti5 + cr5; tr5 = zr5 * zr5; ti5 = zi5 * zi5;
        zi6 = (zr6 + zr6) * zi6 + ci; zr6 = tr6 - ti6 + cr6; tr6 = zr6 * zr6; ti6 = zi6 * zi6;
        zi7 = (zr7 + zr7) * zi7 + ci; zr7 = tr7 - ti7 + cr7; tr7 = zr7 * zr7; ti7 = zi7 * zi7;
        zi0 = (zr0 + zr0) * zi0 + ci; zr0 = tr0 - ti0 + cr0; tr0 = zr0 * zr0; ti0 = zi0 * zi0;
        zi1 = (zr1 + zr1) * zi1 + ci; zr1 = tr1 - ti1 + cr1; tr1 = zr1 * zr1; ti1 = zi1 * zi1;
        zi2 = (zr2 + zr2) * zi2 + ci; zr2 = tr2 - ti2 + cr2; tr2 = zr2 * zr2; ti2 = zi2 * zi2;
        zi3 = (zr3 + zr3) * zi3 + ci; zr3 = tr3 - ti3 + cr3; tr3 = zr3 * zr3; ti3 = zi3 * zi3;
        zi4 = (zr4 + zr4) * zi4 + ci; zr4 = tr4 - ti4 + cr4; tr4 = zr4 * zr4; ti4 = zi4 * zi4;
        zi5 = (zr5 + zr5) * zi5 + ci; zr5 = tr5 - ti5 + cr5; tr5 = zr5 * zr5; ti5 = zi5 * zi5;
        zi6 = (zr6 + zr6) * zi6 + ci; zr6 = tr6 - ti6 + cr6; tr6 = zr6 * zr6; ti6 = zi6 * zi6;
        zi7 = (zr7 + zr7) * zi7 + ci; zr7 = tr7 - ti7 + cr7; tr7 = zr7 * zr7; ti7 = zi7 * zi7;
        zi0 = (zr0 + zr0) * zi0 + ci; zr0 = tr0 - ti0 + cr0; tr0 = zr0 * zr0; ti0 = zi0 * zi0;
        zi1 = (zr1 + zr1) * zi1 + ci; zr1 = tr1 - ti1 + cr1; tr1 = zr1 * zr1; ti1 = zi1 * zi1;
        zi2 = (zr2 + zr2) * zi2 + ci; zr2 = tr2 - ti2 + cr2; tr2 = zr2 * zr2; ti2 = zi2 * zi2;
        zi3 = (zr3 + zr3) * zi3 + ci; zr3 = tr3 - ti3 + cr3; tr3 = zr3 * zr3; ti3 = zi3 * zi3;
        zi4 = (zr4 + zr4) * zi4 + ci; zr4 = tr4 - ti4 + cr4; tr4 = zr4 * zr4; ti4 = zi4 * zi4;
        zi5 = (zr5 + zr5) * zi5 + ci; zr5 = tr5 - ti5 + cr5; tr5 = zr5 * zr5; ti5 = zi5 * zi5;
        zi6 = (zr6 + zr6) * zi6 + ci; zr6 = tr6 - ti6 + cr6; tr6 = zr6 * zr6; ti6 = zi6 * zi6;
        zi7 = (zr7 + zr7) * zi7 + ci; zr7 = tr7 - ti7 + cr7; tr7 = zr7 * zr7; ti7 = zi7 * zi7;
        zi0 = (zr0 + zr0) * zi0 + ci; zr0 = tr0 - ti0 + cr0; tr0 = zr0 * zr0; ti0 = zi0 * zi0;
        zi1 = (zr1 + zr1) * zi1 + ci; zr1 = tr1 - ti1 + cr1; tr1 = zr1 * zr1; ti1 = zi1 * zi1;
        zi2 = (zr2 + zr2) * zi2 + ci; zr2 = tr2 - ti2 + cr2; tr2 = zr2 * zr2; ti2 = zi2 * zi2;
        zi3 = (zr3 + zr3) * zi3 + ci; zr3 = tr3 - ti3 + cr3; tr3 = zr3 * zr3; ti3 = zi3 * zi3;
        zi4 = (zr4 + zr4) * zi4 + ci; zr4 = tr4 - ti4 + cr4; tr4 = zr4 * zr4; ti4 = zi4 * zi4;
        zi5 = (zr5 + zr5) * zi5 + ci; zr5 = tr5 - ti5 + cr5; tr5 = zr5 * zr5; ti5 = zi5 * zi5;
        zi6 = (zr6 + zr6) * zi6 + ci; zr6 = tr6 - ti6 + cr6; tr6 = zr6 * zr6; ti6 = zi6 * zi6;
        zi7 = (zr7 + zr7) * zi7 + ci; zr7 = tr7 - ti7 + cr7; tr7 = zr7 * zr7; ti7 = zi7 * zi7;
        zi0 = (zr0 + zr0) * zi0 + ci; zr0 = tr0 - ti0 + cr0; tr0 = zr0 * zr0; ti0 = zi0 * zi0;
        zi1 = (zr1 + zr1) * zi1 + ci; zr1 = tr1 - ti1 + cr1; tr1 = zr1 * zr1; ti1 = zi1 * zi1;
        zi2 = (zr2 + zr2) * zi2 + ci; zr2 = tr2 - ti2 + cr2; tr2 = zr2 * zr2; ti2 = zi2 * zi2;
        zi3 = (zr3 + zr3) * zi3 + ci; zr3 = tr3 - ti3 + cr3; tr3 = zr3 * zr3; ti3 = zi3 * zi3;
        zi4 = (zr4 + zr4) * zi4 + ci; zr4 = tr4 - ti4 + cr4; tr4 = zr4 * zr4; ti4 = zi4 * zi4;
        zi5 = (zr5 + zr5) * zi5 + ci; zr5 = tr5 - ti5 + cr5; tr5 = zr5 * zr5; ti5 = zi5 * zi5;
        zi6 = (zr6 + zr6) * zi6 + ci; zr6 = tr6 - ti6 + cr6; tr6 = zr6 * zr6; ti6 = zi6 * zi6;
        zi7 = (zr7 + zr7) * zi7 + ci; zr7 = tr7 - ti7 + cr7; tr7 = zr7 * zr7; ti7 = zi7 * zi7;
        zi0 = (zr0 + zr0) * zi0 + ci; zr0 = tr0 - ti0 + cr0; tr0 = zr0 * zr0; ti0 = zi0 * zi0;
        zi1 = (zr1 + zr1) * zi1 + ci; zr1 = tr1 - ti1 + cr1; tr1 = zr1 * zr1; ti1 = zi1 * zi1;
        zi2 = (zr2 + zr2) * zi2 + ci; zr2 = tr2 - ti2 + cr2; tr2 = zr2 * zr2; ti2 = zi2 * zi2;
        zi3 = (zr3 + zr3) * zi3 + ci; zr3 = tr3 - ti3 + cr3; tr3 = zr3 * zr3; ti3 = zi3 * zi3;
        zi4 = (zr4 + zr4) * zi4 + ci; zr4 = tr4 - ti4 + cr4; tr4 = zr4 * zr4; ti4 = zi4 * zi4;
        zi5 = (zr5 + zr5) * zi5 + ci; zr5 = tr5 - ti5 + cr5; tr5 = zr5 * zr5; ti5 = zi5 * zi5;
        zi6 = (zr6 + zr6) * zi6 + ci; zr6 = tr6 - ti6 + cr6; tr6 = zr6 * zr6; ti6 = zi6 * zi6;
        zi7 = (zr7 + zr7) * zi7 + ci; zr7 = tr7 - ti7 + cr7; tr7 = zr7 * zr7; ti7 = zi7 * zi7;
        zi0 = (zr0 + zr0) * zi0 + ci; zr0 = tr0 - ti0 + cr0; tr0 = zr0 * zr0; ti0 = zi0 * zi0;
        zi1 = (zr1 + zr1) * zi1 + ci; zr1 = tr1 - ti1 + cr1; tr1 = zr1 * zr1; ti1 = zi1 * zi1;
        zi2 = (zr2 + zr2) * zi2 + ci; zr2 = tr2 - ti2 + cr2; tr2 = zr2 * zr2; ti2 = zi2 * zi2;
        zi3 = (zr3 + zr3) * zi3 + ci; zr3 = tr3 - ti3 + cr3; tr3 = zr3 * zr3; ti3 = zi3 * zi3;
        zi4 = (zr4 + zr4) * zi4 + ci; zr4 = tr4 - ti4 + cr4; tr4 = zr4 * zr4; ti4 = zi4 * zi4;
        zi5 = (zr5 + zr5) * zi5 + ci; zr5 = tr5 - ti5 + cr5; tr5 = zr5 * zr5; ti5 = zi5 * zi5;
        zi6 = (zr6 + zr6) * zi6 + ci; zr6 = tr6 - ti6 + cr6; tr6 = zr6 * zr6; ti6 = zi6 * zi6;
        zi7 = (zr7 + zr7) * zi7 + ci; zr7 = tr7 - ti7 + cr7; tr7 = zr7 * zr7; ti7 = zi7 * zi7;
        let all = (tr0 + ti0 > 4.0) and (tr1 + ti1 > 4.0) and (tr2 + ti2 > 4.0) and (tr3 + ti3 > 4.0)
            and (tr4 + ti4 > 4.0) and (tr5 + ti5 > 4.0) and (tr6 + ti6 > 4.0) and (tr7 + ti7 > 4.0);
        if all { return 0 }
        i = i + 1;
    }
    var bits: UInt8 = 0;
    if tr0 + ti0 <= 4.0 { bits = bits | 128; }
    if tr1 + ti1 <= 4.0 { bits = bits | 64; }
    if tr2 + ti2 <= 4.0 { bits = bits | 32; }
    if tr3 + ti3 <= 4.0 { bits = bits | 16; }
    if tr4 + ti4 <= 4.0 { bits = bits | 8; }
    if tr5 + ti5 <= 4.0 { bits = bits | 4; }
    if tr6 + ti6 <= 4.0 { bits = bits | 2; }
    if tr7 + ti7 <= 4.0 { bits = bits | 1; }
    bits
}

@main
func main() {
    let args = Process.arguments();
    let n = if args.count > 1 { Int64(parsing: args(1)) ?? 200 } else { 200 };
    let rowBytes = (n + 7) / 8;
    let inv = 2.0 / Float64(from: n);
    // Real parts for every column, padded to a multiple of 8.
    var cr0 = Array[Float64](repeating: 0.0, count: rowBytes * 8);
    for x in 0..<(rowBytes * 8) {
        unsafe cr0(unchecked: x) = Float64(from: x) * inv - 1.5;
    }
    var out = Array[UInt8](repeating: 0, count: rowBytes * n);
    let crs = cr0.asSlice();
    let o = out.asMutableSlice();
    for y in 0..<n {
        let ci = Float64(from: y) * inv - 1.0;
        let base = y * rowBytes;
        for xb in 0..<rowBytes {
            unsafe o(unchecked: base + xb) = renderByte(crs, xb * 8, ci);
        }
        // Mask off padding pixels past the right edge.
        if n % 8 != 0 {
            let keep = UInt8(truncating: (0xFF << (8 - n % 8)) & 0xFF);
            unsafe o(unchecked: base + rowBytes - 1) = unsafe o(unchecked: base + rowBytes - 1) & keep;
        }
    }
    var stdout = StandardOutput();
    let _ = stdout.write(text: "P4\n\(n) \(n)\n");
    let _ = stdout.write(from: out.asSlice());
    let _ = stdout.flush();
}
