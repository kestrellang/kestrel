// test: execution
// stdlib: true
// expect-exit: 3

module Test
import std.num.Int64
import std.core._ExpressibleByArrayLiteral

struct MyList: _ExpressibleByArrayLiteral {
    type Element = Int64
    var count: Int64

    init(_arrayLiteralPointer: lang.ptr[Int64], _arrayLiteralCount: lang.i64) {
        self.count = Int64(intLiteral: _arrayLiteralCount)
    }
}

func main() -> lang.i64 {
    let list: MyList = [1, 2, 3];
    list.count.raw
}
