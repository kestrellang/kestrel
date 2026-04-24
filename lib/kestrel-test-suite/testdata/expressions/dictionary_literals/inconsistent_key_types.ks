// test: diagnostics
// stdlib: true

module Main

func test() {
    let d: [String: Int] = ["key": 1, 42: 2]; // ERROR: does not conform to protocol
}
