// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test clear()
            var dict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);
            let _ = dict.insert(3, 30);
            dict.clear();
            if dict.count != 0 { return 1 }
            if dict.isEmpty == false { return 2 }

            // Test update(key:with:) - key exists
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);
            let updated = dict.update(1, with: { (v) in v * 10 });
            if updated == false { return 3 }
            if dict(1).unwrap() != 100 { return 4 }

            // Test update(key:with:) - key missing
            let notUpdated = dict.update(99, with: { (v) in v * 10 });
            if notUpdated { return 5 }

            // Test upsert(key:default:with:) - key exists
            dict.upsert(2, default: 0, with: { (v) in v + 5 });
            if dict(2).unwrap() != 25 { return 6 }

            // Test upsert(key:default:with:) - key missing
            dict.upsert(99, default: 0, with: { (v) in v + 5 });
            if dict(99).unwrap() != 5 { return 7 }

            // Test merge()
            var base = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = base.insert(1, 10);
            let _ = base.insert(2, 20);
            var other = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = other.insert(2, 200);
            let _ = other.insert(3, 300);
            base.merge(other, uniquingKeysWith: { (old, new) in old + new });
            if base.count != 3 { return 8 }
            if base(1).unwrap() != 10 { return 9 }
            if base(2).unwrap() != 220 { return 10 }
            if base(3).unwrap() != 300 { return 11 }

            // Test retain(matching:)
            var dict2 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = dict2.insert(1, 10);
            let _ = dict2.insert(2, 20);
            let _ = dict2.insert(3, 30);
            let _ = dict2.insert(4, 40);
            dict2.retain(matching: { (k, v) in v > 15 });
            if dict2.count != 3 { return 12 }
            if dict2.contains(1) { return 13 }
            if dict2.contains(2) == false { return 14 }

            // Test removeAll(matching:)
            var dict3 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
            let _ = dict3.insert(1, 10);
            let _ = dict3.insert(2, 20);
            let _ = dict3.insert(3, 30);
            dict3.removeAll(matching: { (k, v) in v >= 20 });
            if dict3.count != 1 { return 15 }
            if dict3(1).unwrap() != 10 { return 16 }

            0
        }
