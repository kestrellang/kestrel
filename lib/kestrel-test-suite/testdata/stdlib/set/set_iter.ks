// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var s = std.collections.Set[std.numeric.Int64]();
            let _ = s.insert(10);
            let _ = s.insert(20);
            let _ = s.insert(30);

            // Iterate and collect sum to verify all elements are visited
            var sum: std.numeric.Int64 = 0;
            var iter = s.iter();
            while let .Some(elem) = iter.next() {
                sum = sum + elem;
            }
            if sum != 60 { return 1 }

            // Verify iter count
            var count: std.numeric.Int64 = 0;
            var iter2 = s.iter();
            while let .Some(_) = iter2.next() {
                count = count + 1;
            }
            if count != 3 { return 2 }

            // Empty set iteration
            let empty = std.collections.Set[std.numeric.Int64]();
            var iter3 = empty.iter();
            if let .Some(_) = iter3.next() {
                return 3
            }

            0
        }
