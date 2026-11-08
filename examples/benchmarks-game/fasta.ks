module Main

import std.system.Process
import std.streams.StandardOutput

let IM: Int64 = 139968;
let IA: Int64 = 3877;
let IC: Int64 = 29573;
let LINE: Int64 = 60;

// Output goes through one big byte buffer that's flushed when nearly full.
struct Out {
    var buf: Array[UInt8]
    var len: Int64
    var stdout: StandardOutput

    init() {
        self.buf = Array[UInt8](repeating: 0, count: 1 << 16);
        self.len = 0;
        self.stdout = StandardOutput();
    }

    mutating func flush() {
        let _ = self.stdout.write(from: self.buf(0..<self.len));
        self.len = 0;
    }

    mutating func reserve(n: Int64) {
        if self.len + n > self.buf.count { self.flush(); }
    }

    mutating func text(s: String) {
        for b in s.bytes { self.reserve(1); unsafe self.buf(unchecked: self.len) = b; self.len = self.len + 1; }
    }
}

// Writes `n` characters of `seq`, repeated cyclically, in lines of 60.
func repeatFasta(mutating out: Out, seq: Array[UInt8], n: Int64) {
    let len = seq.count;
    // Doubled copy so each line can be copied from one contiguous run.
    var src = Array[UInt8](repeating: 0, count: len + LINE);
    for i in 0..<(len + LINE) { unsafe src(unchecked: i) = unsafe seq(unchecked: i % len); }
    var pos: Int64 = 0;
    var left = n;
    while left > 0 {
        let k = if left < LINE { left } else { LINE };
        out.reserve(k + 1);
        let o = out.buf.asMutableSlice();
        let base = out.len;
        for i in 0..<k { unsafe o(unchecked: base + i) = unsafe src(unchecked: pos + i); }
        unsafe o(unchecked: base + k) = 10;
        out.len = base + k + 1;
        pos = pos + k;
        if pos >= len { pos = pos - len; }
        left = left - k;
    }
}

// Writes `n` random characters drawn from `chars` with probabilities
// `probs`. A table indexed by the generator's state maps it straight to a
// character; it gives exactly the floating-point comparison's answer.
func randomFasta(mutating out: Out, chars: Array[UInt8], probs: Array[Float64], n: Int64, mutating seed: Int64) {
    var cum = Array[Float64](repeating: 0.0, count: probs.count);
    var acc = 0.0;
    for i in 0..<probs.count { acc = acc + probs(i); cum(i) = acc; }
    var table = Array[UInt8](repeating: 0, count: IM);
    let im = Float64(from: IM);
    for s in 0..<IM {
        let r = Float64(from: s) / im;
        var j: Int64 = 0;
        while j < cum.count - 1 and r >= unsafe cum(unchecked: j) { j = j + 1; }
        unsafe table(unchecked: s) = unsafe chars(unchecked: j);
    }
    let t = table.asSlice();
    var left = n;
    var sd = seed;
    while left > 0 {
        let k = if left < LINE { left } else { LINE };
        out.reserve(k + 1);
        let o = out.buf.asMutableSlice();
        let base = out.len;
        for i in 0..<k {
            sd = (sd * IA + IC) % IM;
            unsafe o(unchecked: base + i) = unsafe t(unchecked: sd);
        }
        unsafe o(unchecked: base + k) = 10;
        out.len = base + k + 1;
        left = left - k;
    }
    seed = sd;
}

func bytes(s: String) -> Array[UInt8] {
    var a = Array[UInt8]();
    for b in s.bytes { a.append(b); }
    a
}

@main
func main() {
    let args = Process.arguments();
    let n = if args.count > 1 { Int64(parsing: args(1)) ?? 1000 } else { 1000 };
    let alu = bytes("GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA");
    let iubChars = bytes("acgtBDHKMNRSVWY");
    var iubProbs = Array[Float64]();
    for p in [0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02] { iubProbs.append(p); }
    let hsChars = bytes("acgt");
    var hsProbs = Array[Float64]();
    for p in [0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008] { hsProbs.append(p); }

    var out = Out();
    var seed: Int64 = 42;
    out.text(">ONE Homo sapiens alu\n");
    repeatFasta(out, alu, n * 2);
    out.text(">TWO IUB ambiguity codes\n");
    randomFasta(out, iubChars, iubProbs, n * 3, seed);
    out.text(">THREE Homo sapiens frequency\n");
    randomFasta(out, hsChars, hsProbs, n * 5, seed);
    out.flush();
    let _ = out.stdout.flush();
}
