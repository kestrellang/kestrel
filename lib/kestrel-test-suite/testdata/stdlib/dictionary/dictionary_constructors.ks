// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test init(capacity:)
            var dict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64](capacity: 16);
            if dict.count != 0 { return 1 }
            if dict.isEmpty == false { return 2 }
            if dict.capacity < 16 { return 3 }

            // Test that capacity dict works normally after inserts
             dict.insert(1, 100);
             dict.insert(2, 200);
            if dict.count != 2 { return 4 }
            if dict(1).unwrap() != 100 { return 5 }
            if dict(2).unwrap() != 200 { return 6 }

            // Test init(capacity: 0) creates empty dictionary
            var dict2 = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64](capacity: 0);
            if dict2.isEmpty == false { return 7 }
             dict2.insert(5, 50);
            if dict2(5).unwrap() != 50 { return 8 }

            0
        }
