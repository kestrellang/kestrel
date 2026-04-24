// test: execution
// stdlib: true

module Test

struct Data {
    let a: std.num.Int64
    let b: std.num.Int64
    let c: std.num.Int64
}

func main() -> lang.i64 {
    let d = Data(a: 10, b: 20, c: 12);
    if d.a + d.b + d.c != 42 { return 1 }
    0
}
