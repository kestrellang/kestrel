// test: execution
// stdlib: true

module Test

func add(in value: std.numeric.Int64, to base: std.numeric.Int64) -> std.numeric.Int64 {
    base + value
}

func lookup(for key: std.numeric.Int64) -> std.numeric.Int64 {
    key * 2
}

// Overloaded by keyword label
func process(if flag: std.core.Bool) -> std.numeric.Int64 { 1 }
func process(or fallback: std.numeric.Int64) -> std.numeric.Int64 { fallback }

@main
func main() -> lang.i64 {
    if add(in: 10, to: 32) != 42 { return 1 }
    if lookup(for: 5) != 10 { return 2 }
    if process(if: true) != 1 { return 3 }
    if process(or: 99) != 99 { return 4 }
    0
}
