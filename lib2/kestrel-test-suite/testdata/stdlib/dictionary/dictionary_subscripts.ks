// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);

            // Test subscript(key:default:) - key exists
            let val1 = dict(1, default: 999);
            if val1 != 10 { return 1 }

            // Test subscript(key:default:) - key missing, returns default
            let val2 = dict(99, default: 999);
            if val2 != 999 { return 2 }

            // Test that default is NOT inserted
            if dict.contains(99) { return 3 }

            // Test subscript(unwrap:) - key exists
            let val5 = dict(unwrap: 2);
            if val5 != 20 { return 4 }

            0
        }
