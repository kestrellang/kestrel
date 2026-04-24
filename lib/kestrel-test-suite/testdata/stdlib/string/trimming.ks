// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- Mutating trim() ----
            var s1: std.text.String = "  hello  ";
            s1.trim();
            if s1.equals("hello") == false { return 1 }

            // ---- Mutating trimStart() ----
            var s2: std.text.String = "  hello  ";
            s2.trimStart();
            if s2.equals("hello  ") == false { return 2 }

            // ---- Mutating trimEnd() ----
            var s3: std.text.String = "  hello  ";
            s3.trimEnd();
            if s3.equals("  hello") == false { return 3 }

            // ---- Mutating trim(matching:) ----
            var s4: std.text.String = "xxhelloxx";
            s4.trim(matching: { (c) in c.equals('x') });
            if s4.equals("hello") == false { return 4 }

            // ---- Mutating trimStart(matching:) ----
            var s5: std.text.String = "xxhelloxx";
            s5.trimStart(matching: { (c) in c.equals('x') });
            if s5.equals("helloxx") == false { return 5 }

            // ---- Mutating trimEnd(matching:) ----
            var s6: std.text.String = "xxhelloxx";
            s6.trimEnd(matching: { (c) in c.equals('x') });
            if s6.equals("xxhello") == false { return 6 }

            // ---- Non-mutating trimmedStart() ----
            let s7: std.text.String = "  hello  ";
            let ts = s7.trimmedStart();
            if ts.equals("hello  ") == false { return 7 }
            // Original unchanged
            if s7.byteCount != 9 { return 8 }

            // ---- Non-mutating trimmedEnd() ----
            let s8: std.text.String = "  hello  ";
            let te = s8.trimmedEnd();
            if te.equals("  hello") == false { return 9 }

            // ---- Non-mutating trimmed(matching:) ----
            let s9: std.text.String = "..hello..";
            let tm = s9.trimmed(matching: { (c) in c.equals('.') });
            if tm.equals("hello") == false { return 10 }

            // ---- Non-mutating trimmedStart(matching:) ----
            let s10: std.text.String = "..hello..";
            let tsm = s10.trimmedStart(matching: { (c) in c.equals('.') });
            if tsm.equals("hello..") == false { return 11 }

            // ---- Non-mutating trimmedEnd(matching:) ----
            let s11: std.text.String = "..hello..";
            let tem = s11.trimmedEnd(matching: { (c) in c.equals('.') });
            if tem.equals("..hello") == false { return 12 }

            // ---- Trim with leading/trailing whitespace including newlines ----
            var s12: std.text.String = "  hello  ";
            s12.trimStart();
            s12.trimEnd();
            if s12.equals("hello") == false { return 13 }

            // ---- Trim on all-whitespace string ----
            var s13: std.text.String = "   ";
            s13.trim();
            if s13.isEmpty == false { return 14 }

            0
        }
