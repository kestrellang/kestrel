module Main

import std.streams.(StandardInput)
import std.collections.(Dictionary, FoldHasher)

// std's FoldHasher (foldhash): one widening multiply per key. A custom
// multiplicative hasher measured the same; lookups are memory-bound.
type Counts = Dictionary[UInt64, Int64, FoldHasher];

func readAllStdin() -> Array[UInt8] {
    var data = Array[UInt8]();
    data.reserveCapacity(1 << 26);
    var chunk = Array[UInt8](repeating: 0, count: 1 << 20);
    var input = StandardInput();
    loop {
        let n = match input.read(into: chunk.asMutableSlice()) {
            .Ok(n) => n,
            .Err(_) => 0,
        };
        if n == 0 { break }
        data.append(contentsOf: chunk(0..<n));
    }
    data
}

// Counts every k-mer of length `k` in `seq` (2-bit codes).
func countFrames(seq: Slice[UInt8], k: Int64) -> Counts {
    var counts = Counts(capacity: 1 << 16);
    let one: UInt64 = 1;
    let mask: UInt64 = if k == 32 { 0xFFFFFFFFFFFFFFFF } else { (one << (2 * k)) - 1 };
    var key: UInt64 = 0;
    for i in 0..<(k - 1) { key = (key << 2) | UInt64(from: unsafe seq(unchecked: i)); }
    for i in (k - 1)..<seq.count {
        key = ((key << 2) | UInt64(from: unsafe seq(unchecked: i))) & mask;
        if let .Some(&mutating v) = counts(key) {
            v = v + 1;
        } else {
            let _ = counts.insert(key, 1);
        }
    }
    counts
}

func encode(s: String) -> UInt64 {
    var key: UInt64 = 0;
    for b in s.bytes { key = (key << 2) | UInt64(from: codeOf(b)); }
    key
}

func codeOf(b: UInt8) -> UInt8 {
    match b {
        65 or 97 => 0,   // A
        67 or 99 => 1,   // C
        71 or 103 => 2,  // G
        _ => 3,          // T
    }
}

func decode(key: UInt64, k: Int64) -> String {
    var s = "";
    var i = k - 1;
    while i >= 0 {
        let c = (key >> (2 * i)) & 3;
        s.append(char: if c == 0 { 'A' } else if c == 1 { 'C' } else if c == 2 { 'G' } else { 'T' });
        i = i - 1;
    }
    s
}

func printFrequencies(seq: Slice[UInt8], k: Int64) {
    let counts = countFrames(seq, k);
    var entries = Array[(UInt64, Int64)]();
    for (key, v) in counts { entries.append((key, v)); }
    // Most frequent first; ties broken by key order (which is ACGT order).
    entries.sort(by: { (a: (UInt64, Int64), b: (UInt64, Int64)) in if a.1 != b.1 { a.1 > b.1 } else { a.0 < b.0 } });
    let total = Float64(from: seq.count - k + 1);
    for (key, v) in entries {
        println("\(decode(key, k)) \(Float64(from: v) * 100.0 / total:.3)");
    }
    println();
}

func printCount(seq: Slice[UInt8], fragment: String) {
    let k = fragment.bytes.count;
    let counts = countFrames(seq, k);
    let n = counts(encode(fragment)) ?? 0;
    println("\(n)\t\(fragment)");
}

@main
func main() {
    let data = readAllStdin();
    let input = data.asSlice();
    // Find the ">THREE" header, then gather the sequence after it as 2-bit codes.
    var i: Int64 = 0;
    let len = input.count;
    loop {
        if i + 6 > len { break }
        if unsafe input(unchecked: i) == 62 and unsafe input(unchecked: i + 1) == 84 and unsafe input(unchecked: i + 2) == 72 { break }
        // Skip to next line.
        while i < len and unsafe input(unchecked: i) != 10 { i = i + 1; }
        i = i + 1;
    }
    while i < len and unsafe input(unchecked: i) != 10 { i = i + 1; }
    i = i + 1;
    var seqArr = Array[UInt8](repeating: 0, count: len - i);
    let seqOut = seqArr.asMutableSlice();
    var n: Int64 = 0;
    while i < len {
        let b = unsafe input(unchecked: i);
        if b != 10 {
            unsafe seqOut(unchecked: n) = codeOf(b);
            n = n + 1;
        }
        i = i + 1;
    }
    let seq = &seqArr(0..<n);

    printFrequencies(seq, 1);
    printFrequencies(seq, 2);
    for f in ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"] {
        printCount(seq, f);
    }
}
