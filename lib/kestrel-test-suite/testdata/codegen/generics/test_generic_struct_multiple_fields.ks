// test: execution
// stdlib: true

module Test

struct Pair[A, B] {
    let first: A
    let second: B
}

func main() -> lang.i64 {
    let p = Pair[std.num.Int64, std.num.Int64](first: 40, second: 2);
    if p.first + p.second != 42 { return 1 }
    0
}
