// test: diagnostics

module Test

func test(x x: std.num.Int64) -> std.text.String {
    "value: " + (x + 1).format()
}
