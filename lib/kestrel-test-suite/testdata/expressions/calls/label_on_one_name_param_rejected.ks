// test: diagnostics
// stdlib: true

// A parameter declared with one name takes no label: the name is internal.
// Writing it as a label (a common habit from Swift) gets a targeted error
// that names the parameter, for free functions and methods alike.

module Test

func greet(name: String) -> String { name }

struct Joiner {
    func join(separator: String) -> String { separator }
}

func main() {
    let _ = greet(name: "x"); // ERROR: 'name' is the parameter's name, not an argument label
    let _ = Joiner().join(separator: ","); // ERROR: 'separator' is the parameter's name, not an argument label
}
