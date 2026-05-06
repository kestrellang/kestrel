// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- compare() ----
            let a: std.text.String = "apple";
            let b: std.text.String = "banana";
            let cmp = a.compare(b);
            if cmp != std.core.Ordering.Less { return 1 }

            let cmp2 = b.compare(a);
            if cmp2 != std.core.Ordering.Greater { return 2 }

            let cmp3 = a.compare(a);
            if cmp3 != std.core.Ordering.Equal { return 3 }

            // ---- clone() ----
            let original: std.text.String = "hello";
            let cloned = original.clone();
            if cloned.isEqual(to: "hello") == false { return 4 }

            // clone is COW - mutating clone doesn't affect original
            var mClone = original.clone();
            mClone.append(" world");
            if original.byteCount != 5 { return 5 }
            if mClone.byteCount != 11 { return 6 }

            // ---- add() ----
            let s1: std.text.String = "hello";
            let s2: std.text.String = " world";
            let combined = s1.add(s2);
            if combined.isEqual(to: "hello world") == false { return 7 }
            // Originals unchanged
            if s1.byteCount != 5 { return 8 }
            if s2.byteCount != 6 { return 9 }

            0
        }
