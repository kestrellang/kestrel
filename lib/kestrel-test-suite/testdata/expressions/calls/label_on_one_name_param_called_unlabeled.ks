// test: execution
// stdlib: true
// expect-stdout: x\na,b\n

// The fix the label-on-one-name-param error suggests: call positionally.

module Test

func greet(name: String) -> String { name }

@main
func main() {
    println(greet("x"));
    println(["a", "b"].joined(","));
}
