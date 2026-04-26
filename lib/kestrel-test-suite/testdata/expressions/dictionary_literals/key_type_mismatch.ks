// test: diagnostics
// stdlib: true

module Main

func test() {
    let d: [String: Int] = [42: 1]; // ERROR: does not conform to protocol
}
