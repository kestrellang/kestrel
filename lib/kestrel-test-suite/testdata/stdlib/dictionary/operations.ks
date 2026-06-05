// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();

            // Test isEmpty initially
            if dict.isEmpty == false { return 1 }

            // Test insert and count
            let _ = dict.insert(1, 100);
            let _ = dict.insert(2, 200);
            if dict.count != 2 { return 2 }

            // Test contains
            if dict.contains(1) == false { return 3 }
            if dict.contains(999) { return 4 }

            // Test subscript access
            let val = dict(2);
            if val.isNone() { return 5 }
            if val.unwrap() != 200 { return 6 }

            // Test remove
            let removed = dict.remove(1);
            if removed.isNone() { return 7 }
            if removed.unwrap() != 100 { return 8 }
            if dict.count != 1 { return 9 }

            0
        }
