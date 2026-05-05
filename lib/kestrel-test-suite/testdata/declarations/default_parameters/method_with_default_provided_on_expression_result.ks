// test: diagnostics

module Test

func test(x x: std.numeric.Int64) -> std.text.String {
    (x + 1).formatted(std.text.FormatOptions.default())
}
