module Main

import std.system.Process

// The permutation lives in one UInt64, 4 bits per element (n <= 16), so a
// prefix flip is a few register ops instead of a loop over memory:
// reverse all 16 nibbles (byte swap, then swap the nibbles in each byte),
// shift the reversed prefix down, and splice it onto the untouched suffix.

func reverseNibbles(x: UInt64) -> UInt64 {
    let y = x.byteSwapped;
    ((y & 0x0F0F0F0F0F0F0F0F) << 4) | ((y >> 4) & 0x0F0F0F0F0F0F0F0F)
}

// Mask of the low `k` nibbles (k in 1...16).
func lowMask(k: Int64) -> UInt64 {
    if k >= 16 { return 0xFFFFFFFFFFFFFFFF }
    if k <= 0 { return 0 }
    let one: UInt64 = 1;
    (one << (4 * k)) - 1
}

// Rotates the first `k` elements left by one: element 0 moves to k - 1.
func rotatePrefix(x: UInt64, k: Int64) -> UInt64 {
    let m = lowMask(k);
    let p = x & m;
    (x & (m ^ 0xFFFFFFFFFFFFFFFF)) | (p >> 4) | ((p & 0xF) << (4 * (k - 1)))
}

@main
func main() {
    let args = Process.arguments();
    let n = if args.count > 1 { Int64(parsing: args(1)) ?? 7 } else { 7 };
    var countArr = Array[Int64](repeating: 0, count: 32);
    let count = countArr.asMutableSlice();

    // Identity permutation: element i in nibble i.
    var perm: UInt64 = 0;
    for i in 0..<n { perm = perm | (UInt64(truncating: i) << (4 * i)); }

    var total: Int64 = 1;
    for i in 1..=n { total = total * i; }

    // keep(k) clears the low k nibbles: the part a flip of length k leaves alone.
    var keepArr = Array[UInt64](repeating: 0, count: 17);
    for k in 0..<17 { unsafe keepArr(unchecked: k) = lowMask(k) ^ 0xFFFFFFFFFFFFFFFF; }
    let keep = keepArr.asSlice();

    var checksum: Int64 = 0;
    var maxFlips: Int64 = 0;
    var index: Int64 = 0;
    loop {
        var first = Int64(truncating: perm & 0xF);
        if first != 0 {
            var p = perm;
            var flips: Int64 = 0;
            while first != 0 {
                // Reverse elements 0...first.
                let k = first + 1;
                let rev = reverseNibbles(p) >> (4 * (16 - k));
                p = (p & unsafe keep(unchecked: k)) | rev;
                flips = flips + 1;
                first = Int64(truncating: p & 0xF);
            }
            if index & 1 == 0 { checksum = checksum + flips; } else { checksum = checksum - flips; }
            if flips > maxFlips { maxFlips = flips; }
        }
        index = index + 1;
        if index >= total { break }

        // Next permutation in counting order.
        perm = rotatePrefix(perm, 2);
        var i: Int64 = 1;
        loop {
            let c = unsafe count(unchecked: i) + 1;
            unsafe count(unchecked: i) = c;
            if c <= i { break }
            unsafe count(unchecked: i) = 0;
            i = i + 1;
            perm = rotatePrefix(perm, i + 1);
        }
    }
    println("\(checksum)");
    println("Pfannkuchen(\(n)) = \(maxFlips)");
}
