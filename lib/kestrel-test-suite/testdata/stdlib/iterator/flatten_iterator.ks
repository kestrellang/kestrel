// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Flatten nested iterators
            var nested = std.collections.Array[std.collections.Array[std.numeric.Int64]]();
            var inner1 = std.collections.Array[std.numeric.Int64]();
            inner1.append(1);
            inner1.append(2);
            var inner2 = std.collections.Array[std.numeric.Int64]();
            inner2.append(3);
            inner2.append(4);
            var inner3 = std.collections.Array[std.numeric.Int64]();
            inner3.append(5);
            nested.append(inner1);
            nested.append(inner2);
            nested.append(inner3);

            let flat = nested.iter().map(as: { (arr) in arr.iter() }).flatten().collect();
            if flat.count != 5 { return 1 }
            if flat(unchecked: 0) != 1 { return 2 }
            if flat(unchecked: 4) != 5 { return 3 }

            0
        }
