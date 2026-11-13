// test: execution
// stdlib: true
// expect-stdout: [owned] [   owned  ]\n[owned2+]\n[z]\n[] true\n[Q] [abc]\nhello HeLLo\nSTRASSE\naXYcaXYc ab\n

module Test

// Trims, clear(), lowercase() and replace() work in place on an owned
// buffer, and still leave shared copies alone.
@main
func main() {
    var d = String();
    d.append("   owned  ");
    let dd = d;
    d.trim();
    println("[\(d)] [\(dd)]");
    var e = String();
    e.append("   owned2  ");
    e.trim();
    e.append("+");
    println("[\(e)]");
    var f = "***z**";
    f.trim { (ch) in ch == '*' };
    println("[\(f)]");
    var g = "    ";
    g.trim();
    println("[\(g)] \(g.isEmpty)");
    var h = String();
    h.append("abc");
    let hh = h;
    h.clear();
    h.append("Q");
    println("[\(h)] [\(hh)]");
    var l = "HeLLo";
    let l2 = l;
    l.lowercase();
    println("\(l) \(l2)");
    var up = "straße";
    up.uppercase();
    println(up);
    var r = "abcabc";
    r.replace("b", with: "XY");
    r.replace("zz", with: "!");
    var r2 = "ab";
    r2.replace("", with: "q");
    println("\(r) \(r2)");
}
