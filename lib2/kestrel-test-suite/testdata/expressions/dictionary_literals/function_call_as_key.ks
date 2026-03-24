// test: diagnostics
// stdlib: true

module Main

func getKey() -> String { "computed" }

func test() {
    let d: [String: Int] = [getKey(): 42];
}
