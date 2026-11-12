// test: execution
// stdlib: true
// expect-stdout: hi there\n[hi there  ]\nhi\n

// A `var` holding a slice rebinds on assignment: writing through a shared
// `&` is never legal, so `a = …` can only mean "point at this instead".

module Test

@main
func main() {
    let s = "  hi there  ";
    var a = s.trimmed();
    println(a);
    a = s.trimmedStart();
    println("[\(a)]");
    a = a.subslice(from: 0, to: 2);
    println(a);
}
