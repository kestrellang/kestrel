// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test append
            var s = std.text.String();
            s.append("hello");
            s.append(" world");
            if s.byteCount != 11 { return 1 }

            // Test trim
            let padded: std.text.String = "  hello  ";
            let trimmed = padded.trimmed();
            if trimmed.byteCount != 5 { return 2 }

            // Test lowercase/uppercase
            let mixed: std.text.String = "HeLLo";
            let lower = mixed.lowercased();
            let upper = mixed.uppercased();
            if lower.equals("hello") == false { return 3 }
            if upper.equals("HELLO") == false { return 4 }

            0
        }
