// test: execution
// stdlib: true

module Test

struct Counter {
    var value: std.num.Int64
}

func increment_copy(c: Counter) -> std.num.Int64 {
    // This is a copy, original is not modified
    c.value + 1
}

func main() -> lang.i64 {
    let c = Counter(value: 41);
    let result = increment_copy(c);
    // Result should be 42, but original c.value is still 41
    if result != 42 { return 1 }
    if c.value != 41 { return 2 }
    0
}
