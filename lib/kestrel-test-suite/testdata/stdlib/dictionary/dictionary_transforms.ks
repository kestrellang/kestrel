// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
             dict.insert(1, 10);
             dict.insert(2, 20);
             dict.insert(3, 30);

            // Test mapValues()
            let doubled = dict.mapValues({ (v) in v * 2 });
            if doubled.count != 3 { return 1 }
            if doubled(1).unwrap() != 20 { return 2 }
            if doubled(2).unwrap() != 40 { return 3 }
            if doubled(3).unwrap() != 60 { return 4 }

            // Test compactMapValues()
            // Map values: keep only values > 15 by returning Some/None
            let compacted = dict.compactMapValues({ (v) in
                if v > 15 { .Some(v * 10) } else { .None }
            });
            if compacted.count != 2 { return 5 }
            if compacted.contains(1) { return 6 }
            if compacted(2).unwrap() != 200 { return 7 }
            if compacted(3).unwrap() != 300 { return 8 }

            // Test filter(where:)
            let filtered = dict.filter(where: { (k, v) in v >= 20 });
            if filtered.count != 2 { return 9 }
            if filtered.contains(1) { return 10 }
            if filtered(2).unwrap() != 20 { return 11 }
            if filtered(3).unwrap() != 30 { return 12 }

            // Test merging() - non-mutating
            var other = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
             other.insert(3, 300);
             other.insert(4, 400);
            let merged = dict.merging(other, uniquingKeysWith: { (old, new) in new });
            if merged.count != 4 { return 13 }
            if merged(1).unwrap() != 10 { return 14 }
            if merged(3).unwrap() != 300 { return 15 }
            if merged(4).unwrap() != 400 { return 16 }

            // Original dict unchanged
            if dict.count != 3 { return 17 }
            if dict(3).unwrap() != 30 { return 18 }

            0
        }
