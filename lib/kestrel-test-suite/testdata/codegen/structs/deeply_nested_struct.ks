// test: execution
// stdlib: true

module Test

struct Level3 {
    let value: std.num.Int64
}

struct Level2 {
    let inner: Level3
    let bonus: std.num.Int64
}

struct Level1 {
    let middle: Level2
    let top: std.num.Int64
}

func main() -> lang.i64 {
    let obj = Level1(
        middle: Level2(
            inner: Level3(value: 10),
            bonus: 20
        ),
        top: 12
    );
    // 10 + 20 + 12 = 42
    if obj.middle.inner.value + obj.middle.bonus + obj.top != 42 { return 1 }
    0
}
