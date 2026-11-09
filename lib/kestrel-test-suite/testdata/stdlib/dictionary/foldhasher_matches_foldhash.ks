// test: execution
// stdlib: true
// expect-stdout: 7802958577144658410 9334703032957541393 16786407058030477747 9875149619861430127 17868859424096800208 10059274568297399103 13057922587192178958 6435818663042871643 11706264116630228471 1672865815808568260 2616405782746877038 5926763729393919521 15484580566130583619 8291369128318882795 18156143974145111856 5511930934263097803 1152462073497936510 7986900134327625654 867609654051534841 14243659699540982235 15441421752425041607 9830430025261302893 8455366587011698566 3238439889170295032\n6927092246129628096 14114000593917783084 3684837271291316703 6539499979922275124\n11720782544543684272\n589684135938649225\n

module Test

// `FoldHasher` is foldhash's fast variant with its fixed seeds, so it gives
// the same values as Rust's `foldhash::fast::FixedState::default()` (0.1.5).
// Line 1: one `write(bytes:)` of the bytes `i * 7 + 3` for lengths covering
// every path (empty, 1–3, 4–7, 8–16, medium up to 255, long from 256).
// Line 2: 1, 2, 3 and 5 words through `write(uint64:)`, which buffers two
// words at a time. Line 3: bytes and words mixed. Line 4: no input.

func bytes(n: Int64) -> Array[UInt8] {
    var a: Array[UInt8] = Array();
    var i: Int64 = 0;
    while i < n {
        a.append(UInt8(truncating: i * 7 + 3));
        i = i + 1
    }
    a
}

func word(i: Int64) -> UInt64 {
    UInt64(truncating: i).multiplyWrapping(11400714819323198485).addWrapping(1)
}

@main
func main() {
    var line = "";
    for n in [0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 48, 63, 64, 100, 255, 256, 257, 300, 1000] {
        var h = FoldHasher();
        let b = bytes(n);
        h.write(b.asSlice());
        if line != "" { line = line + " " };
        line = line + "\(h.finish())"
    }
    print(line + "\n");

    var words = "";
    for k in [1, 2, 3, 5] {
        var h = FoldHasher();
        var i: Int64 = 0;
        while i < k {
            h.write(uint64: word(i));
            i = i + 1
        }
        if words != "" { words = words + " " };
        words = words + "\(h.finish())"
    }
    print(words + "\n");

    var m = FoldHasher();
    let hello: Array[UInt8] = [104, 101, 108, 108, 111];  // "hello"
    m.write(hello.asSlice());
    m.write(uint64: 5);
    let forty = bytes(40);
    m.write(forty.asSlice());
    m.write(uint64: 40);
    m.write(uint64: 7);
    print("\(m.finish())\n");

    var empty = FoldHasher();
    print("\(empty.finish())\n");
}
