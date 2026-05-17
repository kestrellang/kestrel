// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- lowercaseAscii() (mutating) ----
            var s1: std.text.String = "Hello WORLD";
            s1.lowercaseAscii();
            if s1.isEqual(to: "hello world") == false { return 1 }

            // ---- uppercaseAscii() (mutating) ----
            var s2: std.text.String = "Hello world";
            s2.uppercaseAscii();
            if s2.isEqual(to: "HELLO WORLD") == false { return 2 }

            // ---- lowercasedAscii() (non-mutating) ----
            let s3: std.text.String = "HELLO";
            let low = s3.lowercasedAscii();
            if low.isEqual(to: "hello") == false { return 3 }
            // Original unchanged
            if s3.isEqual(to: "HELLO") == false { return 4 }

            // ---- uppercasedAscii() (non-mutating) ----
            let s4: std.text.String = "hello";
            let up = s4.uppercasedAscii();
            if up.isEqual(to: "HELLO") == false { return 5 }
            // Original unchanged
            if s4.isEqual(to: "hello") == false { return 6 }

            // ---- titlecased() ----
            let s5: std.text.String = "hello world";
            let titled = s5.titlecased();
            if titled.isEqual(to: "Hello World") == false { return 7 }

            // titlecased with multiple words
            let s6: std.text.String = "the quick brown fox";
            let titled2 = s6.titlecased();
            if titled2.isEqual(to: "The Quick Brown Fox") == false { return 8 }

            // ---- equalsCaseInsensitive() ----
            let a: std.text.String = "Hello";
            let b: std.text.String = "hello";
            let c: std.text.String = "HELLO";
            let d: std.text.String = "world";
            if a.equalsCaseInsensitive(b) == false { return 9 }
            if a.equalsCaseInsensitive(c) == false { return 10 }
            if a.equalsCaseInsensitive(d) { return 11 }

            // ASCII case conversion preserves non-alpha chars
            let s7: std.text.String = "Hello 123!";
            if s7.lowercasedAscii().isEqual(to: "hello 123!") == false { return 12 }
            if s7.uppercasedAscii().isEqual(to: "HELLO 123!") == false { return 13 }

            0
        }
