// test: execution
// stdlib: true
// expect-stdout: 3 [a] [b] [c]\nb\n17\n

module Test

// Iterating `lines` yields zero-copy StringSlices; `toOwned()` keeps one.
@main
func main() {
    let text = "a\nb\r\nc";
    var parts = Array[String]();
    for line in text.lines {
        parts.append(line.toOwned());
    }
    println("\(parts.count) [\(parts(0))] [\(parts(1))] [\(parts(2))]");
    println(text.lines(1));
    var total: Int64 = 0;
    for line in "5\n12".lines {
        total = total + (Int64(parsing: line) ?? 0);
    }
    println(total);
}
