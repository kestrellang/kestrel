// test: diagnostics
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10);
            arr.append(20);
            arr(0) = 99; // ERROR: cannot assign to this expression
            if arr(0) != 99 { return 1 }
            0
        }
