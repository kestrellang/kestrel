// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // ---- repeated(count:) ----
            let s1: std.text.String = "ab";
            let r1 = s1.repeated(3);
            if r1.isEqual(to: "ababab") == false { return 1 }

            // Repeat zero times
            let r0 = s1.repeated(0);
            if r0.isEmpty == false { return 2 }

            // Repeat once
            let r1x = s1.repeated(1);
            if r1x.isEqual(to: "ab") == false { return 3 }

            // ---- pad(leading:with:) ----
            let s2: std.text.String = "hi";
            let ps = s2.pad(leading: 5, with: '0');
            if ps.isEqual(to: "000hi") == false { return 4 }

            // Pad when already long enough
            let s3: std.text.String = "hello";
            let ps2 = s3.pad(leading: 3, with: '0');
            if ps2.isEqual(to: "hello") == false { return 5 }

            // ---- pad(trailing:with:) ----
            let pe = s2.pad(trailing: 5, with: '.');
            if pe.isEqual(to: "hi...") == false { return 6 }

            // Pad end when already long enough
            let pe2 = s3.pad(trailing: 3, with: '.');
            if pe2.isEqual(to: "hello") == false { return 7 }

            // Pad start with space
            let s4: std.text.String = "42";
            let padded = s4.pad(leading: 6, with: ' ');
            if padded.isEqual(to: "    42") == false { return 8 }
            if padded.byteCount != 6 { return 9 }

            // Pad end with space
            let padded2 = s4.pad(trailing: 6, with: ' ');
            if padded2.isEqual(to: "42    ") == false { return 10 }
            if padded2.byteCount != 6 { return 11 }

            0
        }
