// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(10);
            arr.append(20);
            arr.append(30);

            // Test checked subscript (safe access)
            let val = arr(checked: 1);
            if val.isNone() { return 1 }
            if val.unwrap() != 20 { return 2 }

            // Test out of bounds returns None
            let oob = arr(checked: 100);
            if oob.isSome() { return 3 }

            // Test getUnchecked
            if arr(unchecked: 0) != 10 { return 4 }
            if arr(unchecked: 2) != 30 { return 5 }

            0
        }
