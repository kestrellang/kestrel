// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test init() creates an empty set
            let s = std.collections.Set[std.num.Int64]();
            if s.count != 0 { return 1 }
            if s.isEmpty == false { return 2 }
            if s.contains(0) { return 3 }

            0
        }
