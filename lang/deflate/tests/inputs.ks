// Test inputs, built the same way as in make_fixtures.py.

module deflate_tests_fixtures

func pattern(n: Int64) -> Array[UInt8] {
    var out: [UInt8] = [];
    for i in 0..<n { out.append(UInt8(from: i % 251)); }
    out
}

func text(n: Int64) -> Array[UInt8] {
    let words = ["the", "quick", "brown", "fox", "jumps", "over", "lazy", "dogs", "while", "kestrels", "hover", "above"];
    var out: [UInt8] = [];
    var i: Int64 = 0;
    while out.count < n {
        out.append(contentsOf: words((i * 7 + i / 3) % words.count).toBytes());
        out.append(32);
        i = i + 1;
    }
    var trimmed: [UInt8] = [];
    for k in 0..<n { trimmed.append(out(k)); }
    trimmed
}

func noise(n: Int64) -> Array[UInt8] {
    var state: Int64 = 12345;
    var out: [UInt8] = [];
    for _ in 0..<n {
        state = (state * 1103515245 + 12345) & 0x7FFFFFFF;
        out.append(UInt8(from: (state >> 16) & 0xFF));
    }
    out
}

func input(named name: String) -> Array[UInt8] {
    if name == "empty" { return []; }
    if name == "hello" { return "hello, hello, hello!".toBytes(); }
    if name == "hellohello" { return "hello, hello, hello!hello, hello, hello!".toBytes(); }
    if name == "pattern" { return pattern(3000); }
    if name == "text" { return text(8000); }
    if name == "noise" { return noise(1500); }
    Array[UInt8](repeating: 0, count: 100000)
}

func allInputNames() -> Array[String] { ["empty", "hello", "pattern", "text", "noise", "zeros"] }

func fromHex(hex: String) -> Array[UInt8] {
    let digits = hex.toBytes();
    var out: [UInt8] = [];
    var i: Int64 = 0;
    while i + 1 < digits.count {
        out.append(UInt8(from: hexValue(digits(i)) * 16 + hexValue(digits(i + 1))));
        i = i + 2;
    }
    out
}

func hexValue(digit: UInt8) -> Int64 {
    let d = Int64(from: digit);
    if d >= 97 { return d - 87; }
    d - 48
}

func check(name: String, ok: Bool) {
    if ok { print("PASS \(name)\n"); } else { print("FAIL \(name)\n"); }
}
