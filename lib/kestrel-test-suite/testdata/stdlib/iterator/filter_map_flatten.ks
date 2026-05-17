// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test filterMap with optionals
            var arr = std.collections.Array[std.result.Optional[std.numeric.Int64]]();
            arr.append(.Some(1));
            arr.append(.None);
            arr.append(.Some(3));

            let compacted = arr.iter().compactMap().collect();
            if compacted.count != 2 { return 1 }
            if compacted(unchecked: 0) != 1 { return 2 }
            if compacted(unchecked: 1) != 3 { return 3 }

            // Test flatMap
            var nested = std.collections.Array[std.collections.Array[std.numeric.Int64]]();
            var inner1 = std.collections.Array[std.numeric.Int64]();
            inner1.append(1);
            inner1.append(2);
            var inner2 = std.collections.Array[std.numeric.Int64]();
            inner2.append(3);
            nested.append(inner1);
            nested.append(inner2);

            let flat = nested.iter().flatMap({ (arr) in arr.iter() }).collect();
            if flat.count != 3 { return 4 }
            if flat(unchecked: 0) != 1 { return 5 }
            if flat(unchecked: 2) != 3 { return 6 }

            0
        }
