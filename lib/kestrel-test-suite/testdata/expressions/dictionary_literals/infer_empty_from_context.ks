// test: diagnostics
// stdlib: true

module Main

func process(data: [String: Int]) { }

func test() {
    process([:])
}
