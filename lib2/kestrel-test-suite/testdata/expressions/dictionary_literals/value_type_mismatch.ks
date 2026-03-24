// test: diagnostics
// stdlib: true

module Main

func test() {
    let d: [String: Int] = ["key": "value"]; // ERROR: does not conform to protocol
}
