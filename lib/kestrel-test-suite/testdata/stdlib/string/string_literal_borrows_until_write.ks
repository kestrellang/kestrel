// test: execution
// stdlib: true
// expect-stdout: Hello, world|Hello\nabc|ABC\n[q]\n0 true\n[hi there]|[  x]\n

module Test

// A string literal borrows its static bytes (capacity 0) and copies them
// into an owned buffer on the first write. Writes through any path must
// leave the literal, and every other copy of it, untouched.
@main
func main() {
    var s = "Hello";
    let t = s;
    s.append(", world");
    println("\(s)|\(t)");
    var u = "ABC";
    u.lowercaseAscii();
    println("\(u)|\("ABC")");
    var c = "xyz";
    c.clear();
    c.append("q");
    println("[\(c)]");
    let lit = "abc";
    println("\(lit.capacity) \(lit == "abc")");
    var a = "  hi there \n";
    a.trim();
    var b = "  x";
    let b2 = b;
    b.trimStart();
    println("[\(a)]|[\(b2)]");
}
