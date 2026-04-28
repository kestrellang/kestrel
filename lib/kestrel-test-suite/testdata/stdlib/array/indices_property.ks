// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.numeric.Int64]();
            arr.append(10); arr.append(20); arr.append(30);

            // indices should return Range(0, count)
            let idx = arr.indices;
            if idx.start != 0 { return 1 }
            if idx.end != 3 { return 2 }

            // Iterate over indices and access elements
            var sum: std.numeric.Int64 = 0;
            for i in arr.indices {
                sum = sum + arr(i)
            }
            if sum != 60 { return 3 }

            // indices on empty array
            let empty = std.collections.Array[std.numeric.Int64]();
            let emptyIdx = empty.indices;
            if emptyIdx.start != 0 { return 4 }
            if emptyIdx.end != 0 { return 5 }

            // indices on single element array
            var single = std.collections.Array[std.numeric.Int64]();
            single.append(42);
            let singleIdx = single.indices;
            if singleIdx.start != 0 { return 6 }
            if singleIdx.end != 1 { return 7 }

            0
        }
