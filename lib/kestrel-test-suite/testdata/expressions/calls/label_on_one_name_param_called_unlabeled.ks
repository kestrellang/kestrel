// test: execution
// stdlib: true
// expect-stdout: x\n,\n

// The fix the label-on-one-name-param error suggests: call positionally.

module Test

func greet(name: String) -> String { name }

struct Joiner {
    func join(separator: String) -> String { separator }
}

@main
func main() {
    println(greet("x"));
    println(Joiner().join(","));
}
