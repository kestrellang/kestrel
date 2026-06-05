// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(10); arr.append(20); arr.append(30);

            // asSlice() returns a Slice view of the entire array
            let slice = arr.asSlice();
            if slice.count != 3 { return 1 }
            if slice(unchecked: 0) != 10 { return 2 }
            if slice(unchecked: 1) != 20 { return 3 }
            if slice(unchecked: 2) != 30 { return 4 }

            // asSlice on empty array
            let empty = std.collections.Array[std.numeric.Int64]();
            let emptySlice = empty.asSlice();
            if emptySlice.count != 0 { return 5 }

            0
        }
