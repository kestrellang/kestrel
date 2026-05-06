// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var a = std.collections.Set[std.numeric.Int64]();
            let _ = a.insert(1);
            let _ = a.insert(2);
            let _ = a.insert(3);

            // Same elements, different insertion order
            var b = std.collections.Set[std.numeric.Int64]();
            let _ = b.insert(3);
            let _ = b.insert(1);
            let _ = b.insert(2);

            if a.isEqual(to: b) == false { return 1 }

            // Different sizes
            var c = std.collections.Set[std.numeric.Int64]();
            let _ = c.insert(1);
            let _ = c.insert(2);

            if a.isEqual(to: c) { return 2 }

            // Different elements, same size
            var d = std.collections.Set[std.numeric.Int64]();
            let _ = d.insert(1);
            let _ = d.insert(2);
            let _ = d.insert(4);

            if a.isEqual(to: d) { return 3 }

            // Both empty
            let e1 = std.collections.Set[std.numeric.Int64]();
            let e2 = std.collections.Set[std.numeric.Int64]();
            if e1.isEqual(to: e2) == false { return 4 }

            0
        }
