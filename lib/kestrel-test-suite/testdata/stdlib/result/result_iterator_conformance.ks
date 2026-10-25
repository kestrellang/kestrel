// test: execution
// stdlib: true

module Test

        // ResultIterator must conform to Iterator, so the iterator
        // adapters (map/collect etc.) apply to Result.iter().
        @main
        func main() -> lang.i64 {
            let ok: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Ok(21);
            let doubled: std.collections.Array[std.numeric.Int64] = ok.iter().map(as: { (x) in x * 2 }).collect();
            if doubled.count != 1 { return 1 }
            if doubled(0) != 42 { return 2 }

            let err: std.result.Result[std.numeric.Int64, std.numeric.Int64] = .Err(9);
            let empty: std.collections.Array[std.numeric.Int64] = err.iter().map(as: { (x) in x * 2 }).collect();
            if empty.count != 0 { return 3 }

            0
        }
