// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var a = std.collections.Set[std.numeric.Int64]();
             a.insert(1);
             a.insert(2);
             a.insert(3);

            // Same elements, different insertion order
            var b = std.collections.Set[std.numeric.Int64]();
             b.insert(3);
             b.insert(1);
             b.insert(2);

            if a.isEqual(to: b) == false { return 1 }

            // Different sizes
            var c = std.collections.Set[std.numeric.Int64]();
             c.insert(1);
             c.insert(2);

            if a.isEqual(to: c) { return 2 }

            // Different elements, same size
            var d = std.collections.Set[std.numeric.Int64]();
             d.insert(1);
             d.insert(2);
             d.insert(4);

            if a.isEqual(to: d) { return 3 }

            // Both empty
            let e1 = std.collections.Set[std.numeric.Int64]();
            let e2 = std.collections.Set[std.numeric.Int64]();
            if e1.isEqual(to: e2) == false { return 4 }

            0
        }
