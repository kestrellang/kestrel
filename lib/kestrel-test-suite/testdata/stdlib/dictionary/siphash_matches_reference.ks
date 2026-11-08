// test: execution
// stdlib: true
// expect-stdout: 12370263754033579228 14552427766023178899 15245385783003124032 3931806377309739662\n2712449776846519780 15213397504630561110 14722228812742953830 11309216583640668172\n14168811326635809549\ntrue true true\n

module Test

// `DefaultHasher` is SipHash-1-3. With the reference key 00 01 .. 0f,
// `DefaultHasher(key0:key1:)` gives the reference SipHash-1-3 value of the
// bytes 00, 01, .. for lengths 0, 1, 7, 8, 9, 15, 16 and 17, and of a word
// written with `write(uint64:)` followed by 3 bytes (line 3). Line 4:
// split writes hash like one write, the default hasher agrees with itself
// within a run, and the unkeyed `FastHasher` is still available.
@main
func main() {
    let k0: UInt64 = 506097522914230528;
    let k1: UInt64 = 1084818905618843912;
    var line = "";
    var n = 0;
    for len in [0, 1, 7, 8, 9, 15, 16, 17] {
        var bytes: Array[UInt8] = [];
        var i = 0;
        while i < len { bytes.append(UInt8(truncating: i)); i = i + 1 }
        var h = DefaultHasher(key0: k0, key1: k1);
        h.write(bytes.asSlice());
        line.append("\(h.finish())");
        n = n + 1;
        line.append(if n == 4 { "\n" } else if n == 8 { "\n" } else { " " });
    }
    print(line);
    var w = DefaultHasher(key0: k0, key1: k1);
    w.write(uint64: 123456789);
    let tailBytes: Array[UInt8] = [1, 2, 3];
    w.write(tailBytes.asSlice());
    print("\(w.finish())\n");

    let all: Array[UInt8] = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110];
    var one = DefaultHasher(key0: k0, key1: k1);
    one.write(all.asSlice());
    var split = DefaultHasher(key0: k0, key1: k1);
    split.write(all.asSlice()(0..<3));
    split.write(all.asSlice()(3..<11));
    var d1 = DefaultHasher();
    "key".hash(into: d1);
    var d2 = DefaultHasher();
    "key".hash(into: d2);
    var f1 = FastHasher();
    "key".hash(into: f1);
    var f2 = FastHasher();
    "key".hash(into: f2);
    print("\(one.finish() == split.finish()) \(d1.finish() == d2.finish()) \(f1.finish() == f2.finish())\n");
}
