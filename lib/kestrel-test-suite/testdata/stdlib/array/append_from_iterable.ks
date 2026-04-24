// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // appendFrom(iterable:) with a range
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(2);
            arr.appendFrom(std.core.Range[std.num.Int64](3, 6));
            if arr.count != 5 { return 1 }
            if arr(0) != 1 { return 2 }
            if arr(1) != 2 { return 3 }
            if arr(2) != 3 { return 4 }
            if arr(3) != 4 { return 5 }
            if arr(4) != 5 { return 6 }

            // appendFrom with empty range
            arr.appendFrom(std.core.Range[std.num.Int64](0, 0));
            if arr.count != 5 { return 7 }

            // appendFrom on empty array
            var emptyArr = std.collections.Array[std.num.Int64]();
            emptyArr.appendFrom(std.core.Range[std.num.Int64](10, 13));
            if emptyArr.count != 3 { return 8 }
            if emptyArr(0) != 10 { return 9 }
            if emptyArr(1) != 11 { return 10 }
            if emptyArr(2) != 12 { return 11 }

            0
        }
