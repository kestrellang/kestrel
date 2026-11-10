// test: execution
// stdlib: true
// expect-stdout: Some(6) Some(3) Some(1) [   7]\n

// ByteIndex, CharIndex and GraphemeIndex print as their byte offset, and
// honor width like an Int64.
module Test

@main
func main() -> Int64 {
    let s = "héllo, world";
    let comma = s.firstIndex(of: ",");
    let l = s.chars.firstIndex(of: 'l');
    let e = s.graphemes.firstIndex(where: { it.chars.first == .Some('é') });
    println("\(comma) \(l) \(e) [\(ByteIndex(7):>4)]");
    0
}
