// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // merge(from:) with another dictionary's iter
            var dict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);

            var other = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = other.insert(2, 200);
            let _ = other.insert(3, 300);

            dict.merge(from:other, uniquingKeysWith: { (old, new) in old + new });
            if dict.count != 3 { return 1 }
            if dict(1).unwrap() != 10 { return 2 }
            if dict(2).unwrap() != 220 { return 3 }
            if dict(3).unwrap() != 300 { return 4 }

            // merge(from:) with "take new" strategy
            var dict2 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = dict2.insert(1, 10);

            var src = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = src.insert(1, 99);
            let _ = src.insert(2, 200);

            dict2.merge(from:src, uniquingKeysWith: { (old, new) in new });
            if dict2.count != 2 { return 5 }
            if dict2(1).unwrap() != 99 { return 6 }
            if dict2(2).unwrap() != 200 { return 7 }

            // merge(from:) with empty source - no change
            var dict3 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = dict3.insert(1, 10);
            let emptySrc = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            dict3.merge(from:emptySrc, uniquingKeysWith: { (old, new) in new });
            if dict3.count != 1 { return 8 }
            if dict3(1).unwrap() != 10 { return 9 }

            0
        }
