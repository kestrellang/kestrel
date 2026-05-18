// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- Mutating trim() ----
            var s1: std.text.String = "  hello  ";
            s1.trim();
            if s1.isEqual(to: "hello") == false { return 1 }

            // ---- Mutating trimStart() ----
            var s2: std.text.String = "  hello  ";
            s2.trimStart();
            if s2.isEqual(to: "hello  ") == false { return 2 }

            // ---- Mutating trimEnd() ----
            var s3: std.text.String = "  hello  ";
            s3.trimEnd();
            if s3.isEqual(to: "  hello") == false { return 3 }

            // ---- Mutating trim(where:) ----
            var s4: std.text.String = "xxhelloxx";
            s4.trim(where: { (c) in c.isEqual(to: 'x') });
            if s4.isEqual(to: "hello") == false { return 4 }

            // ---- Mutating trimStart(where:) ----
            var s5: std.text.String = "xxhelloxx";
            s5.trimStart(where: { (c) in c.isEqual(to: 'x') });
            if s5.isEqual(to: "helloxx") == false { return 5 }

            // ---- Mutating trimEnd(where:) ----
            var s6: std.text.String = "xxhelloxx";
            s6.trimEnd(where: { (c) in c.isEqual(to: 'x') });
            if s6.isEqual(to: "xxhello") == false { return 6 }

            // ---- Non-mutating trimmedStart() ----
            let s7: std.text.String = "  hello  ";
            let ts = s7.trimmedStart();
            if ts.toOwned().isEqual(to: "hello  ") == false { return 7 }
            // Original unchanged
            if s7.byteCount != 9 { return 8 }

            // ---- Non-mutating trimmedEnd() ----
            let s8: std.text.String = "  hello  ";
            let te = s8.trimmedEnd();
            if te.toOwned().isEqual(to: "  hello") == false { return 9 }

            // ---- Non-mutating trimmed(where:) ----
            let s9: std.text.String = "..hello..";
            let tm = s9.trimmed(where: { (c) in c.isEqual(to: '.') });
            if tm.toOwned().isEqual(to: "hello") == false { return 10 }

            // ---- Non-mutating trimmedStart(where:) ----
            let s10: std.text.String = "..hello..";
            let tsm = s10.trimmedStart(where: { (c) in c.isEqual(to: '.') });
            if tsm.toOwned().isEqual(to: "hello..") == false { return 11 }

            // ---- Non-mutating trimmedEnd(where:) ----
            let s11: std.text.String = "..hello..";
            let tem = s11.trimmedEnd(where: { (c) in c.isEqual(to: '.') });
            if tem.toOwned().isEqual(to: "..hello") == false { return 12 }

            // ---- Trim with leading/trailing whitespace including newlines ----
            var s12: std.text.String = "  hello  ";
            s12.trimStart();
            s12.trimEnd();
            if s12.isEqual(to: "hello") == false { return 13 }

            // ---- Trim on all-whitespace string ----
            var s13: std.text.String = "   ";
            s13.trim();
            if s13.isEmpty == false { return 14 }

            0
        }
