// test: diagnostics

module Test

func test(x x: std.numeric.Int64) -> std.text.String {
    "value: " + (x + 1).format()
}
