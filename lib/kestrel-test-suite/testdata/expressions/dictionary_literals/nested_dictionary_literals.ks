// test: diagnostics
// stdlib: true

module Main

func getNested() -> [String: [String: Int]] {
    ["outer": ["inner": 42]]
}
