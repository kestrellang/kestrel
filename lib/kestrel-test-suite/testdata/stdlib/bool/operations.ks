// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let t: std.core.Bool = true;
            let f: std.core.Bool = false;

            // Test and (uses closure-based logicalAnd)
            if (t and t) == false { return 1 }
            if t and f { return 2 }

            // Test or (uses closure-based logicalOr)
            if (t or f) == false { return 3 }
            if f or f { return 4 }

            // Test logicalNot
            if t.logicalNot() { return 5 }
            if f.logicalNot() == false { return 6 }

            // Test equals
            if t.isEqual(to: t) == false { return 7 }
            if t.isEqual(to: f) { return 8 }

            0
        }
