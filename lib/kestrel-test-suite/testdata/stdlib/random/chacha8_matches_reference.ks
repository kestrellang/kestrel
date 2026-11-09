// test: execution
// stdlib: true
// expect-stdout: 15438444565445410878 5891172212252782290\n7387829322293334546 14283834251474640193 9360588686666418927\ntrue true 36\n

module Test

// `ChaCha8` is the ChaCha stream cipher with 8 rounds, 64-bit block counter
// and zero nonce. Line 1 is the all-zero key's first output of blocks 0 and
// 1 (the published ChaCha8 vector starts 3e00ef2f895f40d6, the
// little-endian bytes of the first number). Line 2 is `ChaCha8(seed: 7)`,
// whose key is the seed expanded with SplitMix64; both match an
// independent reference implementation. `DefaultRandom` (behind
// `shuffle()` and `random(in:)`) is a process-wide ChaCha8 keyed from the
// OS once.
@main
func main() {
    var zero = ChaCha8(keyWords: 0, 0, 0, 0);
    let first = zero.nextUInt64();
    var i = 1;
    while i < 8 { let _ = zero.nextUInt64(); i = i + 1 }
    print("\(first) \(zero.nextUInt64())\n");

    var seeded = ChaCha8(seed: 7);
    let a = seeded.nextUInt64();
    let b = seeded.nextUInt64();
    print("\(a) \(b) \(seeded.nextUInt64())\n");

    var d = DefaultRandom();
    let x = d.nextUInt64();
    var values = [1, 2, 3, 4, 5, 6, 7, 8];
    values.shuffle();
    var sum = 0;
    for v in values { sum = sum + v }
    let r = Int64.random(in: 0..<10);
    print("\(x != d.nextUInt64()) \(r >= 0 and r < 10) \(sum)\n");
}
