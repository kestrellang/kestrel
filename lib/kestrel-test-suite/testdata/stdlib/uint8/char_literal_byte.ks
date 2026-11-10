// test: execution
// stdlib: true
// expect-stdout: 35 44 1 true\n2 0 127\n

// A `UInt8` takes an ASCII character literal, so bytes compare and match
// against characters directly: `b == '#'` instead of `b == 35`. A bare
// character literal still defaults to `Char`.
module Test

func count(of target: UInt8, in text: String) -> Int64 {
    var n = 0;
    for b in text.bytes {
        if b == target { n += 1; }
    }
    n
}

@main
func main() -> Int64 {
    let hash: UInt8 = '#';
    let comma: UInt8 = ',';
    let first = match "a,b".bytes(0) { 'a' => 1, _ => 2 };
    println("\(hash) \(comma) \(first) \("a,b".bytes(1) == ',')");
    let c = 'x';
    let nul: UInt8 = '\0';
    let del: UInt8 = '\x7F';
    println("\(count(of: ',', in: "x,y,z")) \(nul) \(del)");
    if c != 'x' { return 1; }
    0
}
