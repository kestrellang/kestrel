// test: execution
// stdlib: true
// expect-stdout: hi\nhi  there\n

// The converted `String` is an owned copy: changing the source afterwards
// leaves it alone.

module Test

@main
func main() {
    var s = "  hi  ";
    let owned: String = s.trimmed();
    s.append("there");
    println(owned);
    let t: String = s.trimmed();
    println(t);
}
