// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(2);
            arr.append(3);

            let mySet = std.collections.Set[std.num.Int64](from: arr);
            if mySet.count != 3 { return 1 }

            0
        }
