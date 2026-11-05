// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test clear()
            var dict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
             dict.insert(1, 10);
             dict.insert(2, 20);
             dict.insert(3, 30);
            dict.clear();
            if dict.count != 0 { return 1 }
            if dict.isEmpty == false { return 2 }

            // Update in place - key exists
             dict.insert(1, 10);
             dict.insert(2, 20);
            var updated = false;
            if let .Some(&mutating v) = dict(1) { v = v * 10; updated = true; }
            if updated == false { return 3 }
            if dict(1).unwrap() != 100 { return 4 }

            // Update in place - key missing
            var notUpdated = false;
            if let .Some(&mutating v) = dict(99) { v = v * 10; notUpdated = true; }
            if notUpdated { return 5 }

            // dict(key, default:) write - key exists
            dict(2, default: 0) += 5;
            if dict(2).unwrap() != 25 { return 6 }

            // dict(key, default:) write - key missing inserts the default
            dict(99, default: 0) += 5;
            if dict(99).unwrap() != 5 { return 7 }

            // Test merge()
            var base = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
             base.insert(1, 10);
             base.insert(2, 20);
            var other = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
             other.insert(2, 200);
             other.insert(3, 300);
            base.merge(other, uniquingKeysWith: { (old, new) in old + new });
            if base.count != 3 { return 8 }
            if base(1).unwrap() != 10 { return 9 }
            if base(2).unwrap() != 220 { return 10 }
            if base(3).unwrap() != 300 { return 11 }

            // Test retain(where:)
            var dict2 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
             dict2.insert(1, 10);
             dict2.insert(2, 20);
             dict2.insert(3, 30);
             dict2.insert(4, 40);
            dict2.retain(where: { (k, v) in v > 15 });
            if dict2.count != 3 { return 12 }
            if dict2.contains(1) { return 13 }
            if dict2.contains(2) == false { return 14 }

            // Test removeAll(where:)
            var dict3 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
             dict3.insert(1, 10);
             dict3.insert(2, 20);
             dict3.insert(3, 30);
            dict3.removeAll(where: { (k, v) in v >= 20 });
            if dict3.count != 1 { return 15 }
            if dict3(1).unwrap() != 10 { return 16 }

            0
        }
