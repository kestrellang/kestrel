// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let a: std.text.Char = 'a';
            let b: std.text.Char = 'b';
            let a2: std.text.Char = 'a';

            // ---- equals() ----
            if a.isEqual(to: a2) == false { return 1 }
            if a.isEqual(to: b) { return 2 }

            // ---- compare() ----
            let cmp = a.compare(b);
            if cmp != std.core.Ordering.Less { return 3 }

            let cmp2 = b.compare(a);
            if cmp2 != std.core.Ordering.Greater { return 4 }

            let cmp3 = a.compare(a2);
            if cmp3 != std.core.Ordering.Equal { return 5 }

            // ---- matches() ----
            if a.matches(a2) == false { return 6 }
            if a.matches(b) { return 7 }

            0
        }
