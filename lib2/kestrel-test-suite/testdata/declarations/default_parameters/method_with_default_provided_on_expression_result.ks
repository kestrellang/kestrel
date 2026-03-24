// test: diagnostics

module Test

func test(x x: std.num.Int64) -> std.text.String {
    (x + 1).format(options: std.text.FormatOptions.default())
}
