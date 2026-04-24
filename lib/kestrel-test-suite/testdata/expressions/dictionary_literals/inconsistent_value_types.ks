// test: diagnostics
// stdlib: true

module Main

func test() {
    let d: [String: Int] = ["a": 1, "b": "two"]; // ERROR: does not conform to protocol
}
