// test: execution
// stdlib: true
// expect-stdout: [0, 1, 2, 9, 3, 4, 5, 6]\n9 [0, 1, 2, 3, 4, 5, 6]\n[0, 3, 4, 5, 6]\n[7, 7, 7, 3, 4, 5, 6]\n[7, 8, 4, 5, 6]\n[7, 8, 4, 5, 6, 42] [7, 8, 4, 5, 6]\n[m, n, z, q] [x, y, z] x m\n[s0]\n

module Test

// insert/remove/removeSubrange/replaceSubrange shift with memmove, and
// growing shared storage copies once into the new buffer. Owned elements
// (Strings) must move exactly once and shared copies stay unchanged.
@main
func main() {
    var a = [1, 2, 3, 4, 5];
    a.insert(0, at: 0);
    a.insert(9, at: 3);
    a.insert(6, at: a.count);
    println(a);
    let r = a.remove(at: 3);
    println("\(r) \(a)");
    a.removeSubrange(1..<3);
    println(a);
    a.replaceSubrange(0..<1, with: [7, 7, 7]);
    println(a);
    a.replaceSubrange(1..<4, with: [8]);
    println(a);
    let b = a;
    a.append(42);
    println("\(a) \(b)");
    var ss = ["x", "y", "z"];
    let keep = ss;
    ss.insert("w", at: 1);
    ss.append("q");
    let gone = ss.remove(at: 0);
    ss.removeSubrange(0..<1);
    ss.replaceSubrange(0..<1, with: ["m", "n"]);
    println("\(ss) \(keep) \(gone) \(ss.popFirst() ?? "-")");
    var big = Array[String]();
    for i in 0..<1000 { big.insert("s\(i)", at: 0) }
    while big.count > 1 { let _ = big.remove(at: 0); }
    println(big);
}
